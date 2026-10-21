//! Design tokens of the redesigned main window: the type scale it reads from
//! the `[ui]` face and the sizes of its rows, from the approved mockup.
//! Dialogs and menus still size their text from `[ui]` directly until their
//! own redesign.

use crate::config::FontConfig;

mod dialog;
mod menu;
mod settings;
pub(crate) use dialog::{DialogLook, NoteTone};
pub(crate) use menu::{Header, ItemState, Leading, MenuLook, keycaps, menu_hover};
pub(crate) use settings::{ButtonKind, ButtonSize, SettingsLook, Tone};

/// A host's or space's row.
pub(crate) const ROW_HEIGHT: f32 = 34.;
/// An agent's or tab's row: its name over its kind.
pub(crate) const AGENT_ROW_HEIGHT: f32 = 44.;
/// An agent's row with the details line Herdr's config adds.
pub(crate) const AGENT_DETAILS_HEIGHT: f32 = 60.;
/// How far a selected row's fill keeps from the sidebar's edges.
pub(crate) const SELECTION_INSET: f32 = 8.;
pub(crate) const SELECTION_RADIUS: f32 = 8.;
/// The sidebar's header and the main header beside it.
pub(crate) const HEADER_HEIGHT: f32 = 48.;
/// The status bar along the window's foot, at the default `[ui]` size.
pub(crate) const STATUS_BAR_HEIGHT: f32 = 30.;
/// A square chrome button holding one icon, and its corners.
pub(crate) const ICON_BUTTON: f32 = 28.;
pub(crate) const ICON_BUTTON_RADIUS: f32 = 7.;

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
