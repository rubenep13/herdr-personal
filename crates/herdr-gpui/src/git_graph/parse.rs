//! Bounded parsing of the read scripts' output. Repository text is untrusted:
//! every field is checked, and names and messages lose control characters.

use super::git::{PAGE, Page, Snapshot, WorktreeCounts};
use super::model::{Commit, CommitKind, Ref, RefKind, split_message};
use crate::notifications::safe_text;
use std::collections::{HashMap, HashSet};

/// Longest author name or e-mail kept.
const NAME_LIMIT: usize = 256;
/// Longest ref name kept.
const REF_LIMIT: usize = 1024;
const SUBJECT_LIMIT: usize = 1024;
const DESCRIPTION_LIMIT: usize = 4096;

fn is_oid(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn root(output: &[u8]) -> Option<String> {
    let root = std::str::from_utf8(output).ok()?.trim_end_matches('\n');
    (!root.is_empty() && !root.contains('\n')).then(|| root.to_owned())
}

pub(super) fn snapshot(output: &[u8]) -> Option<Snapshot> {
    let output = std::str::from_utf8(output).ok()?;
    let mut sections = output.splitn(5, "\u{1e}\n");
    let (refs_text, branch, head, stashes, shallow_text) = (
        sections.next()?,
        sections.next()?.trim(),
        sections.next()?.trim(),
        sections.next()?,
        sections.next()?,
    );
    let head = (!head.is_empty()).then(|| head.to_owned());
    if head.as_deref().is_some_and(|head| !is_oid(head)) {
        return None;
    }

    let mut refs = Vec::new();
    for line in refs_text.lines() {
        let [object, kind, peeled, peeled_kind, name] = fields(line, '\t')?;
        if !is_oid(object) {
            return None;
        }
        // A tag of a tree or a blob has no row; neither does a remote's HEAD.
        let target = match (kind, peeled_kind) {
            ("commit", "") => object,
            ("tag", "commit") if is_oid(peeled) => peeled,
            _ => continue,
        };
        let kind = if name.starts_with("refs/heads/") {
            RefKind::Head
        } else if name.starts_with("refs/remotes/") && !name.ends_with("/HEAD") {
            RefKind::Remote
        } else if name.starts_with("refs/tags/") {
            RefKind::Tag
        } else {
            continue;
        };
        refs.push(Ref {
            name: safe_text(name, REF_LIMIT),
            target: target.to_owned(),
            kind,
        });
    }
    if let Some(head) = &head {
        let name = if branch.is_empty() { "HEAD" } else { branch };
        refs.push(Ref {
            name: safe_text(name, REF_LIMIT),
            target: head.clone(),
            kind: RefKind::Current,
        });
    }

    let mut stash_helpers = HashSet::new();
    for (index, line) in stashes.lines().enumerate() {
        let (stash, parents) = line.split_once('\t')?;
        if !is_oid(stash) {
            return None;
        }
        for parent in parents.split_whitespace().skip(1) {
            if !is_oid(parent) {
                return None;
            }
            stash_helpers.insert(parent.to_owned());
        }
        refs.push(Ref {
            name: format!("stash@{{{index}}}"),
            target: stash.to_owned(),
            kind: RefKind::Stash,
        });
    }

    let mut shallow = HashMap::new();
    for line in shallow_text.lines() {
        let mut oids = line.split_whitespace();
        let Some(boundary) = oids.next() else {
            continue;
        };
        let parents: Vec<String> = oids.map(str::to_owned).collect();
        if !is_oid(boundary) || !parents.iter().all(|parent| is_oid(parent)) {
            return None;
        }
        shallow.insert(boundary.to_owned(), parents);
    }

    let mut seen = HashSet::new();
    let tips = head
        .iter()
        .chain(refs.iter().map(|reference| &reference.target))
        .filter(|tip| seen.insert(tip.as_str()))
        .cloned()
        .collect();
    Some(Snapshot {
        refs,
        head,
        tips,
        stash_helpers,
        shallow,
    })
}

/// Exactly `N` fields separated by `separator`.
fn fields<const N: usize>(line: &str, separator: char) -> Option<[&str; N]> {
    let mut parts = line.splitn(N, separator);
    let fields: [&str; N] = std::array::from_fn(|_| parts.next().unwrap_or_default());
    (line.matches(separator).count() == N - 1).then_some(fields)
}

/// One page of `log -z`; at most `PAGE` rows, without the stash helpers.
pub(super) fn page(output: &[u8], snapshot: &Snapshot) -> Option<Page> {
    let output = String::from_utf8_lossy(output);
    let records: Vec<&str> = output
        .split('\0')
        .filter(|record| !record.is_empty())
        .collect();
    let has_more = records.len() > PAGE;
    let stashes: HashSet<&str> = snapshot
        .refs
        .iter()
        .filter(|reference| reference.kind == RefKind::Stash)
        .map(|reference| reference.target.as_str())
        .collect();
    let mut commits = Vec::with_capacity(records.len().min(PAGE));
    for record in records.into_iter().take(PAGE) {
        let mut parts = record.trim_start_matches('\n').splitn(6, '\u{1f}');
        let (oid, parents, author, email, authored, message) = (
            parts.next()?,
            parts.next()?,
            parts.next()?,
            parts.next()?,
            parts.next()?,
            parts.next()?,
        );
        if !is_oid(oid) {
            return None;
        }
        if snapshot.stash_helpers.contains(oid) {
            continue;
        }
        let mut parents: Vec<String> = parents.split_whitespace().map(str::to_owned).collect();
        if !parents.iter().all(|parent| is_oid(parent)) {
            return None;
        }
        if let Some(real) = snapshot.shallow.get(oid) {
            parents.clone_from(real);
        }
        let kind = if stashes.contains(oid) {
            parents.retain(|parent| !snapshot.stash_helpers.contains(parent));
            CommitKind::Stash
        } else {
            CommitKind::Commit
        };
        let (subject, description) = split_message(message);
        commits.push(Commit {
            oid: oid.to_owned(),
            parents,
            author: safe_text(author, NAME_LIMIT),
            email: safe_text(email, NAME_LIMIT),
            authored: authored.parse().ok()?,
            subject: safe_text(&subject, SUBJECT_LIMIT),
            description: safe_text(&description, DESCRIPTION_LIMIT),
            kind,
        });
    }
    Some(Page { commits, has_more })
}

/// Counts from `status --porcelain=v2`: an entry staged and also changed in
/// the working tree counts in both.
pub(super) fn worktree(output: &[u8]) -> WorktreeCounts {
    let mut counts = WorktreeCounts::default();
    for line in output.split(|byte| *byte == b'\n') {
        match line {
            [b'1' | b'2', b' ', index, worktree, b' ', ..] => {
                counts.staged += usize::from(*index != b'.');
                counts.unstaged += usize::from(*worktree != b'.');
            }
            [b'u', b' ', ..] => counts.conflicted += 1,
            [b'?', b' ', ..] => counts.untracked += 1,
            _ => {}
        }
    }
    counts
}
