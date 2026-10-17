//! Reference labels as hgg groups them: one per local branch with its remotes
//! of the same name folded in, the current one first, and only as many as the
//! column fits.

use crate::git_graph::chips::{Chip, ChipKind, build_chips, fit_chips};
use crate::git_graph::model::{Ref, RefKind};

fn reference(name: &str, target: &str, kind: RefKind) -> Ref {
    Ref {
        name: name.into(),
        target: target.into(),
        kind,
    }
}

fn labels(chips: &[Chip]) -> Vec<(&str, ChipKind, bool)> {
    chips
        .iter()
        .map(|chip| (chip.label.as_str(), chip.kind, chip.current))
        .collect()
}

#[test]
fn local_branch_folds_its_remote_at_the_same_commit() {
    let chips = build_chips(&[
        reference("refs/heads/main", "m", RefKind::Head),
        reference("refs/remotes/origin/main", "m", RefKind::Remote),
        reference("main", "m", RefKind::Current),
    ]);
    assert_eq!(
        chips["m"],
        [Chip {
            label: "main".into(),
            kind: ChipKind::Branch,
            current: true,
            remotes: vec!["origin".into()],
            names: vec!["refs/heads/main".into(), "refs/remotes/origin/main".into()],
        }]
    );
}

#[test]
fn remote_elsewhere_keeps_its_own_label() {
    let chips = build_chips(&[
        reference("refs/heads/main", "m", RefKind::Head),
        reference("refs/remotes/origin/main", "o", RefKind::Remote),
        reference("refs/remotes/upstream/main", "u", RefKind::Remote),
    ]);
    assert_eq!(labels(&chips["m"]), [("main", ChipKind::Branch, false)]);
    assert_eq!(labels(&chips["o"]), [("main", ChipKind::Remote, false)]);
    assert_eq!(chips["o"][0].remotes, ["origin"]);
    assert_eq!(
        labels(&chips["u"]),
        [("upstream/main", ChipKind::Remote, false)]
    );
}

#[test]
fn detached_head_gets_a_head_chip() {
    let chips = build_chips(&[
        reference("refs/tags/t", "x", RefKind::Tag),
        reference("HEAD", "x", RefKind::Current),
    ]);
    assert_eq!(
        labels(&chips["x"]),
        [
            ("HEAD", ChipKind::Detached, true),
            ("t", ChipKind::Tag, false)
        ]
    );
    assert_eq!(chips["x"][0].names, ["HEAD"]);
}

#[test]
fn chips_sort_current_first_then_kind() {
    let chips = build_chips(&[
        reference("stash@{0}", "m", RefKind::Stash),
        reference("refs/remotes/origin/feature", "m", RefKind::Remote),
        reference("refs/tags/v1.0.0", "m", RefKind::Tag),
        reference("refs/heads/zz-release", "m", RefKind::Head),
        reference("refs/heads/main", "m", RefKind::Head),
        reference("refs/remotes/origin/main", "m", RefKind::Remote),
        reference("refs/remotes/upstream/main", "m", RefKind::Remote),
        reference("main", "m", RefKind::Current),
    ]);
    assert_eq!(
        labels(&chips["m"]),
        [
            ("main", ChipKind::Branch, true),
            ("zz-release", ChipKind::Branch, false),
            ("v1.0.0", ChipKind::Tag, false),
            ("feature", ChipKind::Remote, false),
            ("stash@{0}", ChipKind::Stash, false),
        ]
    );
    assert_eq!(chips["m"][0].remotes, ["origin", "upstream"]);
    assert_eq!(
        chips["m"][0].names,
        [
            "refs/heads/main",
            "refs/remotes/origin/main",
            "refs/remotes/upstream/main"
        ]
    );
}

#[test]
fn fit_chips_skips_a_long_label_and_counts_the_rest() {
    let badge = |hidden: usize| 10. + hidden.to_string().len() as f32 * 7.;

    let fit = fit_chips(&[60., 40.], 200., badge, 4., 14.);
    assert_eq!((fit.shown, fit.hidden), (vec![0, 1], 0));

    // The 400 px name cannot fit; the tag after it still shows.
    let fit = fit_chips(&[60., 400., 40.], 200., badge, 4., 14.);
    assert_eq!((fit.shown, fit.hidden), (vec![0, 2], 1));

    // Not even the first fits: it shows alone, shortened, beside the badge.
    let fit = fit_chips(&[300., 50.], 120., badge, 4., 14.);
    assert_eq!((fit.shown.as_slice(), fit.hidden), ([0].as_slice(), 1));
    let width = fit.first_max_width.unwrap();
    assert!((24. ..120.).contains(&width), "{width}");

    let fit = fit_chips(&[], 120., badge, 4., 14.);
    assert_eq!((fit.shown, fit.hidden), (Vec::new(), 0));
}
