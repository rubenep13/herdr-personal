//! The graph's columns and widths, shared by every graph tab.

use super::geometry::REFS_DEFAULT;

/// The panel's default width.
pub(crate) const PANEL_DEFAULT: f32 = 520.;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GraphPrefs {
    pub(crate) author: bool,
    pub(crate) date: bool,
    pub(crate) sha: bool,
    pub(crate) refs_width: f32,
    /// `None` fits the graph to its lanes.
    pub(crate) graph_width: Option<f32>,
    pub(crate) panel_width: f32,
}

impl Default for GraphPrefs {
    fn default() -> Self {
        Self {
            author: false,
            date: false,
            sha: false,
            refs_width: REFS_DEFAULT,
            graph_width: None,
            panel_width: PANEL_DEFAULT,
        }
    }
}
