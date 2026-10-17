//! Reference labels for the branch column, ported from hgg's `buildRefChips`
//! (`model.ts`) and `fitChips` (`geometry.ts`).

use super::model::{Ref, RefKind};
use std::collections::HashMap;

/// A label's kind, in the order labels on one commit are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ChipKind {
    Detached,
    Branch,
    Tag,
    Remote,
    Stash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Chip {
    pub(crate) label: String,
    pub(crate) kind: ChipKind,
    pub(crate) current: bool,
    /// Remotes whose branch of the same name points at the same commit.
    pub(crate) remotes: Vec<String>,
    /// Every full refname the label stands for.
    pub(crate) names: Vec<String>,
}

struct RemoteRef<'a> {
    name: &'a str,
    short: &'a str,
    remote: &'a str,
    branch: &'a str,
    used: bool,
}

/// Groups refs by the commit they point to. A local branch absorbs the remote
/// branches of the same name at its commit, and the current branch sorts
/// first.
pub(crate) fn build_chips(refs: &[Ref]) -> HashMap<String, Vec<Chip>> {
    let mut by_target: HashMap<&str, Vec<&Ref>> = HashMap::new();
    for reference in refs {
        by_target
            .entry(reference.target.as_str())
            .or_default()
            .push(reference);
    }
    by_target
        .into_iter()
        .filter_map(|(target, list)| {
            let chips = chips_of(&list);
            (!chips.is_empty()).then(|| (target.to_owned(), chips))
        })
        .collect()
}

fn of_kind<'a>(list: &[&'a Ref], kind: RefKind) -> impl Iterator<Item = &'a Ref> {
    list.iter()
        .copied()
        .filter(move |reference| reference.kind == kind)
}

fn chips_of(list: &[&Ref]) -> Vec<Chip> {
    let current = of_kind(list, RefKind::Current).next();
    let mut remotes: Vec<RemoteRef> = of_kind(list, RefKind::Remote)
        .map(|reference| {
            let short = reference
                .name
                .strip_prefix("refs/remotes/")
                .unwrap_or(&reference.name);
            let (remote, branch) = short.split_once('/').unwrap_or(("", short));
            RemoteRef {
                name: &reference.name,
                short,
                remote,
                branch,
                used: false,
            }
        })
        .collect();
    let mut chips = Vec::new();
    for reference in of_kind(list, RefKind::Head) {
        let label = reference
            .name
            .strip_prefix("refs/heads/")
            .unwrap_or(&reference.name);
        let mut names = vec![reference.name.clone()];
        let mut tracking = Vec::new();
        for remote in remotes
            .iter_mut()
            .filter(|remote| !remote.used && remote.branch == label)
        {
            remote.used = true;
            tracking.push(remote.remote.to_owned());
            names.push(remote.name.to_owned());
        }
        chips.push(Chip {
            label: label.to_owned(),
            kind: ChipKind::Branch,
            current: current.is_some_and(|current| current.name == label),
            remotes: tracking,
            names,
        });
    }
    if let Some(current) = current
        && (current.name == "HEAD" || !chips.iter().any(|chip| chip.current))
    {
        chips.push(Chip {
            label: current.name.clone(),
            kind: ChipKind::Detached,
            current: true,
            remotes: Vec::new(),
            names: vec!["HEAD".into()],
        });
    }
    for reference in of_kind(list, RefKind::Tag) {
        chips.push(single(
            reference
                .name
                .strip_prefix("refs/tags/")
                .unwrap_or(&reference.name),
            ChipKind::Tag,
            &reference.name,
        ));
    }
    for remote in remotes.iter().filter(|remote| !remote.used) {
        // As GitKraken does, a remote branch shows by its branch name; only a
        // remote other than origin keeps its name, to tell them apart.
        let label = if remote.remote == "origin" {
            remote.branch
        } else {
            remote.short
        };
        let mut chip = single(label, ChipKind::Remote, remote.name);
        chip.remotes.push(remote.remote.to_owned());
        chips.push(chip);
    }
    for reference in of_kind(list, RefKind::Stash) {
        chips.push(single(&reference.name, ChipKind::Stash, &reference.name));
    }
    chips.sort_by_key(|chip| (!chip.current, chip.kind));
    chips
}

fn single(label: &str, kind: ChipKind, name: &str) -> Chip {
    Chip {
        label: label.to_owned(),
        kind,
        current: false,
        remotes: Vec::new(),
        names: vec![name.to_owned()],
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChipFit {
    /// Indices of the labels shown, in order.
    pub(crate) shown: Vec<usize>,
    /// How many the `+N` badge counts.
    pub(crate) hidden: usize,
    /// The width the first label is shortened to, when it does not fit whole.
    pub(crate) first_max_width: Option<f32>,
}

/// Which labels fit in the column, in priority order, keeping room for a
/// `+N` badge and a short leader to the graph. A label too wide to fit is
/// skipped so later ones can still show; the first one always shows,
/// shortened if needed.
pub(crate) fn fit_chips(
    widths: &[f32],
    available: f32,
    badge_width: impl Fn(usize) -> f32,
    gap: f32,
    min_leader: f32,
) -> ChipFit {
    let space = available - min_leader;
    let total = widths.len();
    let mut shown: Vec<usize> = Vec::new();
    let mut used = 0.;
    for (index, width) in widths.iter().enumerate() {
        let next = used + if shown.is_empty() { 0. } else { gap } + width;
        // Room for a badge counting every label not shown so far or after this one.
        let not_shown = total - shown.len() - 1;
        let reserve = if not_shown > 0 {
            gap + badge_width(not_shown)
        } else {
            0.
        };
        if next + reserve <= space {
            used = next;
            shown.push(index);
        }
    }
    let Some(&first) = widths.first() else {
        return ChipFit {
            shown,
            hidden: 0,
            first_max_width: None,
        };
    };
    if shown.first() == Some(&0) {
        return ChipFit {
            hidden: total - shown.len(),
            shown,
            first_max_width: None,
        };
    }
    // The highest-priority label always shows, taking the room it needs.
    let others_width: f32 = shown.iter().map(|index| widths[*index] + gap).sum();
    let hidden = total - 1 - shown.len();
    let reserve = if hidden > 0 {
        gap + badge_width(hidden)
    } else {
        0.
    };
    let room = space - others_width - reserve;
    if room >= first.min(60.) {
        shown.insert(0, 0);
        return ChipFit {
            shown,
            hidden,
            first_max_width: (room < first).then_some(room),
        };
    }
    let alone = total - 1;
    let reserve = if alone > 0 {
        gap + badge_width(alone)
    } else {
        0.
    };
    ChipFit {
        shown: vec![0],
        hidden: alone,
        first_max_width: Some((space - reserve).max(24.)),
    }
}
