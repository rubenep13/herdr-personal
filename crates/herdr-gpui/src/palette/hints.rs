//! The palette's footer: each hint's keys, drawn as keys, then what they do.
//! The same hints as ever, in the same order.

pub(super) struct Hint {
    pub(super) keys: &'static [&'static str],
    pub(super) text: &'static str,
}

const MOVE: Hint = Hint {
    keys: &["↑", "↓", "Tab", "⌃N", "⌃P"],
    text: "move",
};
const FILTER: Hint = Hint {
    keys: &["⌃Tab"],
    text: "filter",
};
const SELECT: Hint = Hint {
    keys: &["↵"],
    text: "select",
};
const CLOSE: Hint = Hint {
    keys: &["Esc"],
    text: "close",
};

pub(super) const HINTS: &[Hint] = &[MOVE, FILTER, SELECT, CLOSE];

/// Go To adds its status letters and its preview.
pub(super) const GO_TO_HINTS: &[Hint] = &[
    MOVE,
    FILTER,
    Hint {
        keys: &["⌥B", "⌥W", "⌥I", "⌥D", "⌥A"],
        text: "status",
    },
    Hint {
        keys: &["⌥P"],
        text: "preview",
    },
    SELECT,
    CLOSE,
];
