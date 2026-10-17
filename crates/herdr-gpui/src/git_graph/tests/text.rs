//! The text of a row: author initials and bubble colour, and dates bucketed
//! as GitKraken's time markers are.

use crate::git_graph::text::{author_colour, initials, relative_time};

#[test]
fn initials_take_two_letters() {
    assert_eq!(initials("Ruben Esteban"), "RE");
    assert_eq!(initials("Rubén"), "RU");
    assert_eq!(initials("ada.lovelace"), "AL");
    assert_eq!(initials("  "), "?");
    assert_eq!(initials(""), "?");
}

#[test]
fn author_colour_is_stable_per_author() {
    let ada = author_colour("ada@example.com");
    assert_eq!(ada, author_colour("ada@example.com"));
    assert_ne!(ada, author_colour("grace@example.com"));
    // hsl(h, 42 %, 38 %) with h = |hash| % 360 over UTF-16 units.
    let hash = "ab".encode_utf16().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    });
    let expected = gpui::hsla((hash.unsigned_abs() % 360) as f32 / 360., 0.42, 0.38, 1.);
    assert_eq!(author_colour("ab"), expected);
}

#[test]
fn relative_time_buckets_like_gitkraken() {
    const NOW: i64 = 1_792_152_000;
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    let ago = |seconds: i64| relative_time(NOW - seconds, NOW);
    assert_eq!(ago(20), "just now");
    assert_eq!(ago(MINUTE), "1 minute ago");
    assert_eq!(ago(5 * MINUTE), "5 minutes ago");
    assert_eq!(ago(HOUR), "1 hour ago");
    assert_eq!(ago(3 * HOUR), "3 hours ago");
    assert_eq!(ago(2 * DAY), "2 days ago");
    assert_eq!(ago(10 * DAY), "1 week ago");
    assert_eq!(ago(15 * DAY), "2 weeks ago");
    assert_eq!(ago(40 * DAY), "1 month ago");
    assert_eq!(ago(400 * DAY), "1 year ago");
    assert_eq!(ago(800 * DAY), "2 years ago");
    // A date in the future reads as now; an unknown one shows nothing.
    assert_eq!(ago(-HOUR), "just now");
    assert_eq!(relative_time(0, NOW), "");
}
