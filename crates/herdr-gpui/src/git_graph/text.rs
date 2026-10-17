//! The text of a row, ported from hgg's `dom.ts`: author initials, a stable
//! bubble colour per author, and GitKraken's relative time buckets.

/// Two letters for the author bubble: the first of two words, or the first
/// two of one word; `?` when there is no name.
pub(crate) fn initials(name: &str) -> String {
    let mut words = name
        .split(|c: char| c.is_whitespace() || matches!(c, '.' | '_' | '-'))
        .filter(|word| !word.is_empty());
    let Some(first) = words.next() else {
        return "?".into();
    };
    let letters: String = match words.next() {
        Some(second) => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect(),
        None => first.chars().take(2).collect(),
    };
    letters.to_uppercase()
}

/// A stable colour per author, from the e-mail or else the name:
/// `hsl(|hash| % 360, 42 %, 38 %)`, hashing UTF-16 units as hgg does.
pub(crate) fn author_colour(key: &str) -> gpui::Hsla {
    let hash = key.chars().fold(0i32, |hash, c| {
        // hgg reads the first UTF-16 unit of each code point.
        let unit = c.encode_utf16(&mut [0; 2])[0];
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    });
    let hue = (hash.unsigned_abs() % 360) as f32;
    gpui::hsla(hue / 360., 0.42, 0.38, 1.)
}

/// GitKraken's relative time buckets; rows in the same bucket share one
/// marker. `at` and `now` are seconds since the Unix epoch, and an unknown
/// date (`at <= 0`) shows nothing.
pub(crate) fn relative_time(at: i64, now: i64) -> String {
    if at <= 0 {
        return String::new();
    }
    let seconds = now.saturating_sub(at).max(0);
    let plural = |value: i64, unit: &str| {
        if value == 1 {
            format!("1 {unit} ago")
        } else {
            format!("{value} {unit}s ago")
        }
    };
    if seconds < 60 {
        return "just now".into();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return plural(minutes, "minute");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return plural(hours, "hour");
    }
    let days = hours / 24;
    if days < 7 {
        return plural(days, "day");
    }
    if days < 31 {
        return plural(days / 7, "week");
    }
    if days < 365 {
        // A month of 30.4 days, as hgg counts it.
        return plural(days * 10 / 304, "month");
    }
    plural(days / 365, "year")
}
