#![allow(clippy::unwrap_used)]
use super::*;

const STRIP: [u32; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

fn names(range: std::ops::Range<usize>) -> Vec<String> {
    range.map(|index| format!("theme {index}")).collect()
}

fn loaded(names: &[String]) -> Vec<(String, Option<[u32; 8]>)> {
    names
        .iter()
        .map(|name| (name.clone(), Some(STRIP)))
        .collect()
}

#[test]
fn the_cache_keeps_at_most_64_palettes() {
    let mut cache = ThemeSwatches::default();
    let all = names(0..70);
    let epoch = cache.start().unwrap();
    cache.finish(epoch, loaded(&all));
    let kept = all.iter().filter(|name| cache.get(name).is_some()).count();
    assert_eq!(kept, CACHE_LIMIT);
    assert_eq!(cache.get("theme 0"), None, "the oldest go first");
    assert_eq!(cache.get("theme 69"), Some(Some(STRIP)));
}

#[test]
fn a_stale_epoch_is_dropped() {
    let mut cache = ThemeSwatches::default();
    let epoch = cache.start().unwrap();
    assert_eq!(cache.start(), None, "one load at a time");
    cache.invalidate();
    cache.finish(epoch, loaded(&names(0..1)));
    assert_eq!(cache.get("theme 0"), None);
    assert!(cache.start().is_some(), "the slot is free again");
}

#[test]
fn wanted_skips_cached_names_and_takes_twelve() {
    let mut cache = ThemeSwatches::default();
    let epoch = cache.start().unwrap();
    cache.finish(
        epoch,
        vec![("theme 0".into(), Some(STRIP)), ("theme 1".into(), None)],
    );
    assert_eq!(
        cache.get("theme 1"),
        Some(None),
        "a theme with no palette is known"
    );
    let wanted = cache.wanted(&names(0..30));
    assert_eq!(wanted, names(2..2 + BATCH));
}
