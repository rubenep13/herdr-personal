//! Read-only Git scripts that run on the repository's host, locally or over
//! SSH, through `teleport::Host`.
//!
//! Every script clears inherited `GIT_*` variables and runs Git without
//! optional locks, lazy fetches, hooks, a filesystem monitor, any transport
//! or automatic maintenance, so reading never writes to the repository.

use super::detail::{self, CommitDetail, DIFF_READ, DiffText, MAX_DIFF_LINES};
use super::error::{Error, Result};
use super::model::{Commit, Ref};
use super::parse;
use super::worktree::{self, Section, WorktreeStatus};
use crate::teleport::Host;
use herdr_client::{ScriptLimits, shell_quote};
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::AtomicBool,
    time::Duration,
};

/// Commits per page of history.
pub(crate) const PAGE: usize = 500;
/// Bounds for every script.
pub(crate) const LIMITS: ScriptLimits = ScriptLimits {
    output: 8 * 1024 * 1024,
    idle: Duration::from_secs(20),
};

/// Exit statuses the prelude uses for refusals the graph explains.
const EXIT_NOT_A_REPOSITORY: i32 = 3;
const EXIT_GIT_MISSING: i32 = 127;

/// The refs the history is read from, fixed when it loads so every page
/// walks the same tips.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) refs: Vec<Ref>,
    /// The commit `HEAD` points at; `None` before the first commit.
    pub(crate) head: Option<String>,
    /// Every commit a ref or `HEAD` points at, once each.
    pub(crate) tips: Vec<String>,
    /// The index and untracked commits behind each stash, which are not
    /// shown as rows of their own.
    pub(crate) stash_helpers: HashSet<String>,
    /// The real parents of a shallow clone's boundary commits, which `log`
    /// reports as roots; the graph draws them as lines to missing commits.
    pub(crate) shallow: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Page {
    pub(crate) commits: Vec<Commit>,
    /// Whether another page follows; it starts `PAGE` rows further on.
    pub(crate) has_more: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WorktreeCounts {
    pub(crate) staged: usize,
    pub(crate) unstaged: usize,
    pub(crate) untracked: usize,
    pub(crate) conflicted: usize,
}

/// A repository on a host, by its top-level folder.
#[derive(Debug, Clone)]
pub(crate) struct Repo {
    host: Host,
    root: String,
}

/// Clears inherited Git variables, defines `herdr_git`, and enters `folder`.
fn prelude(folder: &str) -> String {
    format!(
        r#"for name in $(env | sed -n 's/^\(GIT_[A-Za-z0-9_]*\)=.*/\1/p'); do unset "$name"; done
herdr_git() {{ GIT_OPTIONAL_LOCKS=0 GIT_NO_LAZY_FETCH=1 git -c core.fsmonitor=false -c core.hooksPath=/dev/null -c protocol.allow=never -c gc.auto=0 "$@"; }}
command -v git >/dev/null 2>&1 || exit {EXIT_GIT_MISSING}
cd -- {folder} || exit {EXIT_NOT_A_REPOSITORY}
"#,
        folder = shell_quote(folder),
    )
}

const SNAPSHOT: &str = r#"herdr_git for-each-ref --format='%(objectname)%09%(objecttype)%09%(*objectname)%09%(*objecttype)%09%(refname)' refs/heads refs/remotes refs/tags
printf '\036\n'
herdr_git symbolic-ref -q --short HEAD || :
printf '\036\n'
herdr_git rev-parse -q --verify 'HEAD^{commit}' || :
printf '\036\n'
if herdr_git rev-parse -q --verify refs/stash >/dev/null; then
    herdr_git log -g --no-show-signature --format='%H%x09%P' refs/stash
fi
printf '\036\n'
shallow=$(herdr_git rev-parse --git-path shallow)
if [ -f "$shallow" ]; then
    head -n 200 "$shallow" | while read -r oid; do
        printf '%s' "$oid"
        herdr_git cat-file commit "$oid" 2>/dev/null | sed -n '/^$/q; s/^parent / /p' | tr -d '\n'
        printf '\n'
    done
fi
"#;

const WORKTREE: &str = "herdr_git status --porcelain=v2 --branch --untracked-files=all\n";

/// Sets the positional parameters to what `diff-tree` compares for `oid`,
/// already quoted: its first parent and it, or `--root` and it for a root
/// commit.
fn first_parent(oid: &str) -> String {
    format!(
        r#"parent=$(herdr_git rev-parse -q --verify {oid}^1 || :)
if [ -n "$parent" ]; then set -- "$parent" {oid}; else set -- --root {oid}; fi"#
    )
}

/// Runs `body` after the prelude in `folder`, translating the prelude's
/// refusals.
fn run(
    host: &Host,
    folder: &str,
    operation: &'static str,
    body: &str,
    input: &[u8],
    cancelled: &AtomicBool,
) -> Result<Vec<u8>> {
    let script = format!("{}{body}", prelude(folder));
    host.capture_fed(&script, input, LIMITS, cancelled)
        .map_err(|source| match &source {
            herdr_client::Error::ScriptExit { status, .. } => match status.code() {
                Some(EXIT_NOT_A_REPOSITORY) => Error::NotARepository,
                Some(EXIT_GIT_MISSING) => Error::GitMissing,
                _ => Error::Script { operation, source },
            },
            _ => Error::Script { operation, source },
        })
}

impl Repo {
    /// The repository `folder` is in, by its top level.
    pub(crate) fn open(host: Host, folder: &str, cancelled: &AtomicBool) -> Result<Self> {
        let body = format!("herdr_git rev-parse --show-toplevel || exit {EXIT_NOT_A_REPOSITORY}\n");
        let output = run(&host, folder, "rev-parse", &body, &[], cancelled)?;
        let root = parse::root(&output).ok_or(Error::Output {
            operation: "rev-parse",
        })?;
        Ok(Self { host, root })
    }

    pub(crate) fn root(&self) -> &str {
        &self.root
    }

    pub(crate) fn snapshot(&self, cancelled: &AtomicBool) -> Result<Snapshot> {
        let output = run(
            &self.host,
            &self.root,
            "for-each-ref",
            SNAPSHOT,
            &[],
            cancelled,
        )?;
        parse::snapshot(&output).ok_or(Error::Output {
            operation: "for-each-ref",
        })
    }

    /// `PAGE` commits from the snapshot's tips, newest first and never a
    /// parent before its children, after skipping `skip`.
    pub(crate) fn page(
        &self,
        snapshot: &Snapshot,
        skip: usize,
        cancelled: &AtomicBool,
    ) -> Result<Page> {
        if snapshot.tips.is_empty() {
            return Ok(Page::default());
        }
        let body = format!(
            "herdr_git log --date-order --no-color --no-show-signature -z \
             --format='%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%B' --max-count={} --skip={skip} --stdin\n",
            PAGE + 1
        );
        // Tips go on stdin: hundreds of refs would overflow one argument.
        let input = snapshot.tips.join("\n") + "\n";
        let output = run(
            &self.host,
            &self.root,
            "log",
            &body,
            input.as_bytes(),
            cancelled,
        )?;
        parse::page(&output, snapshot).ok_or(Error::Output { operation: "log" })
    }

    /// `oid`'s whole message and the files it changed against its first
    /// parent, or against nothing for a root commit.
    pub(crate) fn commit_detail(&self, oid: &str, cancelled: &AtomicBool) -> Result<CommitDetail> {
        if !parse::is_oid(oid) {
            return Err(Error::Output { operation: "show" });
        }
        let oid = shell_quote(oid);
        let parent = first_parent(&oid);
        let body = format!(
            r#"herdr_git show -s --no-show-signature --no-color -z --format='%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%B' {oid} --
{parent}
herdr_git diff-tree -r -M --no-commit-id -z --raw --numstat "$@"
"#
        );
        let output = run(&self.host, &self.root, "show", &body, &[], cancelled)?;
        detail::commit_detail(&output).ok_or(Error::Output { operation: "show" })
    }

    /// The diff of `path` (renamed from `old_path`) in `oid`, against its
    /// first parent, at most `DIFF_READ` lines of it.
    pub(crate) fn file_diff(
        &self,
        oid: &str,
        path: &str,
        old_path: Option<&str>,
        cancelled: &AtomicBool,
    ) -> Result<DiffText> {
        if !parse::is_oid(oid) {
            return Err(Error::Output {
                operation: "diff-tree",
            });
        }
        let oid = shell_quote(oid);
        let paths = std::iter::once(path)
            .chain(old_path)
            .map(|path| shell_quote(&crate::review::literal(path)))
            .collect::<Vec<_>>()
            .join(" ");
        let parent = first_parent(&oid);
        let body = format!(
            r#"{parent}
herdr_git diff-tree -r -M -p --no-color --src-prefix=a/ --dst-prefix=b/ "$@" -- {paths} | head -n {DIFF_READ}
"#
        );
        let output = run(&self.host, &self.root, "diff-tree", &body, &[], cancelled)?;
        Ok(detail::file_diff(&output, path))
    }

    /// The uncommitted files by section, and the branch's upstream.
    pub(crate) fn worktree_status(&self, cancelled: &AtomicBool) -> Result<WorktreeStatus> {
        let body = "herdr_git status --porcelain=v2 --branch --untracked-files=all -z\n";
        let output = run(&self.host, &self.root, "status", body, &[], cancelled)?;
        worktree::parse(&output).ok_or(Error::Output {
            operation: "status",
        })
    }

    /// `path`'s uncommitted change in `section`: the index against HEAD for
    /// staged, the working tree against the index for unstaged, and the
    /// whole file as added for an untracked or conflicted one. Cut like a
    /// commit's diff.
    pub(crate) fn worktree_diff(
        &self,
        section: Section,
        path: &str,
        old_path: Option<&str>,
        cancelled: &AtomicBool,
    ) -> Result<DiffText> {
        let quoted = shell_quote(path);
        let paths = std::iter::once(path)
            .chain(old_path)
            .map(|path| shell_quote(&crate::review::literal(path)))
            .collect::<Vec<_>>()
            .join(" ");
        let diff = "herdr_git diff -M --no-color --no-ext-diff --no-textconv --src-prefix=a/ --dst-prefix=b/";
        let body = match section {
            Section::Staged => format!("{diff} --cached -- {paths} | head -n {DIFF_READ}\n"),
            Section::Unstaged => format!("{diff} -- {paths} | head -n {DIFF_READ}\n"),
            Section::Untracked | Section::Conflicted => {
                format!("head -n {} -- {quoted}\n", MAX_DIFF_LINES + 1)
            }
        };
        let output = run(&self.host, &self.root, "diff", &body, &[], cancelled)?;
        Ok(match section {
            Section::Staged | Section::Unstaged => detail::file_diff(&output, path),
            Section::Untracked | Section::Conflicted => detail::whole_file(&output, path),
        })
    }

    pub(crate) fn worktree(&self, cancelled: &AtomicBool) -> Result<WorktreeCounts> {
        let output = run(&self.host, &self.root, "status", WORKTREE, &[], cancelled)?;
        Ok(parse::worktree(&output))
    }
}
