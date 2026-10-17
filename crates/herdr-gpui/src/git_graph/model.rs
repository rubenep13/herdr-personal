//! The history the graph shows: commits, child before parent, and the refs
//! that point into them.

/// The oid of the synthetic row for uncommitted changes, above HEAD.
pub(crate) const WORKTREE_OID: &str = "__WORKTREE__";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommitKind {
    Commit,
    Stash,
    Worktree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Commit {
    pub(crate) oid: String,
    pub(crate) parents: Vec<String>,
    pub(crate) author: String,
    pub(crate) email: String,
    /// Seconds since the Unix epoch.
    pub(crate) authored: i64,
    pub(crate) subject: String,
    /// The rest of the message on one line, joined by `" | "`.
    pub(crate) description: String,
    pub(crate) kind: CommitKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefKind {
    Head,
    Remote,
    Tag,
    Stash,
    Current,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ref {
    /// The full refname; for `Current`, the branch's short name, or `HEAD`
    /// when it is detached.
    pub(crate) name: String,
    pub(crate) target: String,
    pub(crate) kind: RefKind,
}

/// Splits a commit message into its subject and the rest of its lines,
/// trimmed and joined by `" | "`, as GitKraken shows them on one row.
pub(crate) fn split_message(message: &str) -> (String, String) {
    let mut lines = message.split('\n');
    let subject = lines.next().unwrap_or_default().trim().to_owned();
    let description = lines
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" | ");
    (subject, description)
}
