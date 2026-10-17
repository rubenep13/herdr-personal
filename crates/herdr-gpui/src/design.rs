//! Design tokens of the redesigned main window: the type scale it reads from
//! the `[ui]` face. Dialogs and menus still size their text from `[ui]`
//! directly until their own redesign.

use crate::config::FontConfig;

/// Text sizes of the main window's chrome, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TypeScale {
    /// Names: hosts, spaces, tabs, agents, the header's title.
    pub(crate) name: f32,
    /// Secondary text: an agent's kind, a branch, the header's path.
    pub(crate) secondary: f32,
    /// Details: an agent's model and summary, a host's load.
    pub(crate) detail: f32,
}

impl TypeScale {
    /// The scale for a `[ui]` face: names one point larger than its size,
    /// so the default 12 gives the mockup's 13, 12 and 11.5.
    pub(crate) fn of(font: &FontConfig) -> Self {
        Self {
            name: font.size + 1.,
            secondary: font.size,
            detail: font.size - 0.5,
        }
    }
}
