//! A commit's detail: its whole message and the files it changed against
//! its first parent (against nothing, for a root commit), and one file's
//! diff. Git's plumbing reads them, so no diff driver, colour or textconv
//! setting gets in, and a diff stops at `MAX_DIFF_LINES`.

use super::{model::Commit, parse};
use crate::{
    notifications::safe_text,
    review::{Body, Lines, highlight::Span},
};
use std::collections::HashMap;

/// Lines of one file's diff shown at most.
pub(crate) const MAX_DIFF_LINES: usize = 4000;
/// Lines of Git's output read for one diff: the shown lines and its headers.
pub(super) const DIFF_READ: usize = MAX_DIFF_LINES + 200;
/// Files listed at most.
const MAX_FILES: usize = 5000;
const PATH_LIMIT: usize = 4096;
const BODY_LIMIT: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileChange {
    /// After the change, cleaned for display.
    pub(crate) path: String,
    /// Where a renamed or copied file was, cleaned.
    pub(crate) old_path: Option<String>,
    /// One of `A M D R C B S ?`.
    pub(crate) kind: char,
    pub(crate) additions: Option<u32>,
    pub(crate) deletions: Option<u32>,
    pub(crate) binary: bool,
    /// Both names as Git wrote them, only ever handed back to Git.
    pub(crate) git_path: String,
    pub(crate) git_old_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommitDetail {
    pub(crate) commit: Commit,
    /// The message after its subject, line breaks kept.
    pub(crate) body: String,
    pub(crate) files: Vec<FileChange>,
}

/// A file's diff as the panel shows it.
#[derive(Debug, Clone, Default)]
pub(crate) struct DiffText {
    pub(crate) lines: Lines,
    /// Each line's syntax colours.
    pub(crate) spans: Vec<Vec<Span>>,
    /// Lines were left out past `MAX_DIFF_LINES`.
    pub(crate) truncated: bool,
    /// Not text: there are no lines to show.
    pub(crate) binary: bool,
}

/// "old → new" for a renamed file, else its path.
pub(crate) fn file_label(change: &FileChange) -> String {
    match &change.old_path {
        Some(old) => format!("{old} \u{2192} {}", change.path),
        None => change.path.clone(),
    }
}

/// "+N −N", with a typographic minus, or "bin" for a binary file.
pub(crate) fn file_counts(change: &FileChange) -> String {
    if change.binary {
        return "bin".into();
    }
    format!(
        "+{} \u{2212}{}",
        change.additions.unwrap_or_default(),
        change.deletions.unwrap_or_default()
    )
}

/// The message after its subject: each line cleaned, blank lines at either
/// end dropped.
fn body(message: &str) -> String {
    let lines: Vec<String> = message
        .split('\n')
        .skip(1)
        .map(|line| safe_text(line.trim_end(), BODY_LIMIT))
        .collect();
    let mut body = lines.join("\n").trim().to_owned();
    if let Some((end, _)) = body.char_indices().nth(BODY_LIMIT) {
        body.truncate(end);
    }
    body
}

/// The letter for a raw status, or `?` for one the panel does not name.
fn letter(status: char) -> char {
    match status {
        'A' | 'M' | 'D' | 'R' | 'C' => status,
        'T' => 'M',
        _ => '?',
    }
}

/// `show -s -z` output followed by `diff-tree -z --raw --numstat`: the
/// commit record ends at the first NUL; the file records follow, read by
/// position, so no file name can pass for a record of another kind.
pub(super) fn commit_detail(output: &[u8]) -> Option<CommitDetail> {
    let output = String::from_utf8_lossy(output);
    let (record, files) = output.split_once('\0')?;
    let (commit, message) = parse::commit(record)?;
    Some(CommitDetail {
        body: body(message),
        commit,
        files: file_changes(files)?,
    })
}

fn file_changes(stream: &str) -> Option<Vec<FileChange>> {
    let mut fields = stream.split('\0');
    let mut files = Vec::new();
    let mut counts: HashMap<String, (Option<u32>, Option<u32>, bool)> = HashMap::new();
    while let Some(field) = fields.next() {
        let field = field.trim_start_matches('\n');
        if field.is_empty() {
            continue;
        }
        if let Some(raw) = field.strip_prefix(':') {
            let parts: Vec<&str> = raw.split(' ').collect();
            let [old_mode, new_mode, _, _, status] = parts.as_slice() else {
                return None;
            };
            let status = status.chars().next()?;
            let (old, path) = if matches!(status, 'R' | 'C') {
                (Some(fields.next()?), fields.next()?)
            } else {
                (None, fields.next()?)
            };
            let submodule = *old_mode == "160000" || *new_mode == "160000";
            if files.len() < MAX_FILES {
                files.push(FileChange {
                    path: safe_text(path, PATH_LIMIT),
                    old_path: old.map(|old| safe_text(old, PATH_LIMIT)),
                    kind: if submodule { 'S' } else { letter(status) },
                    additions: None,
                    deletions: None,
                    binary: false,
                    git_path: path.to_owned(),
                    git_old_path: old.map(str::to_owned),
                });
            }
            continue;
        }
        let mut parts = field.splitn(3, '\t');
        let (added, deleted, path) = (parts.next()?, parts.next()?, parts.next()?);
        let path = if path.is_empty() {
            // A rename names both sides next; it is counted under the new.
            fields.next()?;
            fields.next()?
        } else {
            path
        };
        let binary = added == "-" && deleted == "-";
        counts.insert(
            path.to_owned(),
            (added.parse().ok(), deleted.parse().ok(), binary),
        );
    }
    for file in &mut files {
        if let Some((added, deleted, binary)) = counts.get(&file.git_path) {
            file.additions = *added;
            file.deletions = *deleted;
            file.binary = *binary;
            if *binary && file.kind == 'M' {
                file.kind = 'B';
            }
        }
    }
    Some(files)
}

/// The first `keep` lines of `lines`.
fn first_lines(lines: &Lines, keep: usize) -> Lines {
    let mut kept = Lines::default();
    for line in lines.iter().take(keep) {
        kept.push(line.kind, line.old, line.new, lines.text_of(line));
    }
    kept.finish()
}

/// One file's diff from at most `DIFF_READ` lines of `diff-tree -p`, cut
/// at `MAX_DIFF_LINES` and coloured for `name`. Blocking: it runs on the
/// tab's worker.
pub(super) fn file_diff(output: &[u8], name: &str) -> DiffText {
    let text = String::from_utf8_lossy(output);
    let read = text.lines().count();
    let Some(lines) = parse_files_first(&text) else {
        return DiffText {
            binary: true,
            ..DiffText::default()
        };
    };
    let truncated = read >= DIFF_READ || lines.len() > MAX_DIFF_LINES || lines.truncated;
    let lines = if lines.len() > MAX_DIFF_LINES {
        first_lines(&lines, MAX_DIFF_LINES)
    } else {
        lines
    };
    let spans = crate::review::highlight::colour(name, &lines, 0..lines.len());
    DiffText {
        lines,
        spans,
        truncated,
        binary: false,
    }
}

/// A file shown whole, every line added, from at most `MAX_DIFF_LINES + 1`
/// of its lines; no lines for a binary one. Blocking: it runs on the
/// worker.
pub(super) fn whole_file(output: &[u8], name: &str) -> DiffText {
    if output.contains(&0) {
        return DiffText {
            binary: true,
            ..DiffText::default()
        };
    }
    let text = String::from_utf8_lossy(output);
    // `Lines::added` adds its own hunk header.
    let keep = MAX_DIFF_LINES - 1;
    let truncated = text.lines().count() > keep;
    let kept: String = text
        .lines()
        .take(keep)
        .flat_map(|line| [line, "\n"])
        .collect();
    let lines = Lines::added(&kept);
    let spans = crate::review::highlight::colour(name, &lines, 0..lines.len());
    DiffText {
        lines,
        spans,
        truncated,
        binary: false,
    }
}

/// The first file's lines in a diff, empty for no file; `None` for a
/// binary file.
fn parse_files_first(text: &str) -> Option<Lines> {
    let Some(parsed) = crate::review::parse_files(text).into_iter().next() else {
        return Some(Lines::default());
    };
    match parsed.body {
        Body::Loaded(lines) => Some(Lines::clone(&lines)),
        Body::Binary => None,
        Body::Pending | Body::Large | Body::Failed(_) => Some(Lines::default()),
    }
}
