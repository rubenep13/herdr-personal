//! Searching commits by message, author or e-mail (ignoring case), or by
//! hash prefix. Git filters on the repository's host, so only matching
//! commits travel back, and every part stops at the limit.

use super::{
    error::{Error, Result},
    git::{Repo, Snapshot},
    parse::is_oid,
};
use crate::notifications::{safe_text, unsafe_char};
use herdr_client::shell_quote;
use std::sync::atomic::AtomicBool;

/// Results kept at most; the list says when there were more.
pub(crate) const SEARCH_LIMIT: usize = 200;
/// Characters of a query kept.
const QUERY_LIMIT: usize = 200;
const TEXT_LIMIT: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchHit {
    pub(crate) oid: String,
    pub(crate) subject: String,
    pub(crate) author: String,
    pub(crate) authored: i64,
}

/// "200+ results", "1 result" or "No results".
pub(crate) fn results_label(count: usize, more: bool) -> String {
    match (count, more) {
        (0, _) => "No results".into(),
        (count, true) => format!("{count}+ results"),
        (1, false) => "1 result".into(),
        (count, false) => format!("{count} results"),
    }
}

/// The commits reachable from the snapshot's tips whose message, author or
/// e-mail holds `query`, or whose hash starts with it, newest first: at
/// most `limit`, and whether there were more.
pub(crate) fn search(
    repo: &Repo,
    snapshot: &Snapshot,
    query: &str,
    limit: usize,
    cancelled: &AtomicBool,
) -> Result<(Vec<SearchHit>, bool)> {
    let query: String = query
        .trim()
        .chars()
        .filter(|c| !unsafe_char(*c))
        .take(QUERY_LIMIT)
        .collect();
    if query.is_empty() || snapshot.tips.is_empty() {
        return Ok((Vec::new(), false));
    }
    let wanted = limit + 1;
    let hash = query.len() >= 4 && query.chars().all(|c| c.is_ascii_hexdigit());
    let by_hash = if hash {
        format!(
            r#"printf '%s\n' "$tips" | herdr_git log --format=%H --stdin | grep -i -m {wanted} -e "^$q" || :"#
        )
    } else {
        String::new()
    };
    let body = format!(
        r#"tips=$(cat)
q={query}
found=$({{
printf '%s\n' "$tips" | herdr_git log --date-order --no-show-signature --format=%H -i -F --grep="$q" --max-count={wanted} --stdin || :
printf '%s\n' "$tips" | herdr_git log --date-order --no-show-signature --format=%H -i -F --author="$q" --max-count={wanted} --stdin || :
{by_hash}
}} | awk '!seen[$0]++')
[ -n "$found" ] || exit 0
printf '%s\n' "$found" | herdr_git log --no-walk=sorted --no-show-signature --format='%H%x1f%an%x1f%at%x1f%s' --stdin | head -n {wanted}
"#,
        query = shell_quote(&query),
    );
    let input = snapshot.tips.join("\n") + "\n";
    let output = repo.script("log", &body, input.as_bytes(), cancelled)?;
    let output = String::from_utf8_lossy(&output);
    let mut hits = Vec::new();
    for line in output.lines().filter(|line| !line.is_empty()) {
        let mut fields = line.splitn(4, '\u{1f}');
        let (Some(oid), Some(author), Some(authored), Some(subject)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(Error::Output { operation: "log" });
        };
        if !is_oid(oid) {
            return Err(Error::Output { operation: "log" });
        }
        hits.push(SearchHit {
            oid: oid.to_owned(),
            subject: safe_text(subject, TEXT_LIMIT),
            author: safe_text(author, TEXT_LIMIT),
            authored: authored.parse().unwrap_or_default(),
        });
    }
    let more = hits.len() > limit;
    hits.truncate(limit);
    Ok((hits, more))
}
