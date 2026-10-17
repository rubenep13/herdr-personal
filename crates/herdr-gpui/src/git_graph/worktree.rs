//! The uncommitted changes' detail: the upstream with how far ahead and
//! behind the branch is, and each file in its section, read from one
//! `git status --porcelain=v2 -z`.

use super::detail::FileChange;
use crate::notifications::safe_text;

/// Files listed at most.
const MAX_FILES: usize = 5000;
const PATH_LIMIT: usize = 4096;

/// Where an uncommitted file is, in the order the panel lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    Conflicted,
    Staged,
    Unstaged,
    Untracked,
}

impl Section {
    pub(crate) const ALL: [Self; 4] = [
        Self::Conflicted,
        Self::Staged,
        Self::Unstaged,
        Self::Untracked,
    ];

    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Conflicted => "Conflicts",
            Self::Staged => "Staged",
            Self::Unstaged => "Unstaged",
            Self::Untracked => "Untracked",
        }
    }

    /// The key test selectors and the panel name it by.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Conflicted => "conflicted",
            Self::Staged => "staged",
            Self::Unstaged => "unstaged",
            Self::Untracked => "untracked",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorktreeStatus {
    pub(crate) upstream: Option<String>,
    pub(crate) ahead: u32,
    pub(crate) behind: u32,
    pub(crate) files: Vec<(Section, FileChange)>,
}

/// "origin/main · ↑2 ↓1", the counts only when the branch has moved.
pub(crate) fn upstream_label(status: &WorktreeStatus) -> String {
    match &status.upstream {
        None => "No upstream".into(),
        Some(upstream) if status.ahead == 0 && status.behind == 0 => upstream.clone(),
        Some(upstream) => format!(
            "{upstream} \u{00b7} \u{2191}{} \u{2193}{}",
            status.ahead, status.behind
        ),
    }
}

fn change(path: &str, old: Option<&str>, kind: char) -> FileChange {
    FileChange {
        path: safe_text(path, PATH_LIMIT),
        old_path: old.map(|old| safe_text(old, PATH_LIMIT)),
        kind,
        additions: None,
        deletions: None,
        binary: false,
        git_path: path.to_owned(),
        git_old_path: old.map(str::to_owned),
    }
}

fn letter(status: char) -> char {
    match status {
        'A' | 'M' | 'D' | 'R' | 'C' => status,
        'T' => 'M',
        _ => '?',
    }
}

/// `status --porcelain=v2 --branch --untracked-files=all -z`.
pub(super) fn parse(output: &[u8]) -> Option<WorktreeStatus> {
    let output = String::from_utf8_lossy(output);
    let mut status = WorktreeStatus::default();
    let mut records = output.split('\0');
    let push = |status: &mut WorktreeStatus, section, file| {
        if status.files.len() < MAX_FILES {
            status.files.push((section, file));
        }
    };
    while let Some(record) = records.next() {
        if let Some(header) = record.strip_prefix("# ") {
            if let Some(upstream) = header.strip_prefix("branch.upstream ") {
                status.upstream = Some(safe_text(upstream, PATH_LIMIT));
            } else if let Some(counts) = header.strip_prefix("branch.ab ") {
                let (ahead, behind) = counts.split_once(' ')?;
                status.ahead = ahead.strip_prefix('+')?.parse().ok()?;
                status.behind = behind.strip_prefix('-')?.parse().ok()?;
            }
            continue;
        }
        let Some((kind, rest)) = record.split_at_checked(1) else {
            continue;
        };
        match kind {
            "1" | "2" => {
                let fields: Vec<&str> = rest
                    .trim_start()
                    .splitn(if kind == "1" { 8 } else { 9 }, ' ')
                    .collect();
                let codes: Vec<char> = fields.first()?.chars().collect();
                let [staged, unstaged] = codes.as_slice() else {
                    return None;
                };
                let path = *fields.last()?;
                let old = if kind == "2" {
                    Some(records.next()?)
                } else {
                    None
                };
                if *staged != '.' {
                    let renamed = matches!(staged, 'R' | 'C');
                    push(
                        &mut status,
                        Section::Staged,
                        change(path, old.filter(|_| renamed), letter(*staged)),
                    );
                }
                if *unstaged != '.' {
                    let renamed = matches!(unstaged, 'R' | 'C');
                    push(
                        &mut status,
                        Section::Unstaged,
                        change(path, old.filter(|_| renamed), letter(*unstaged)),
                    );
                }
            }
            "u" => {
                let path = rest.trim_start().splitn(10, ' ').last()?;
                push(&mut status, Section::Conflicted, change(path, None, 'U'));
            }
            "?" => push(
                &mut status,
                Section::Untracked,
                change(rest.trim_start(), None, '?'),
            ),
            _ => {}
        }
    }
    Some(status)
}
