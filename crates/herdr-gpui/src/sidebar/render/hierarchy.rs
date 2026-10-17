//! How the Tree layout shows that a host's spaces belong to it: a deeper
//! indent, a line running down from the host's mark, or one block per host.
//! Each item of the list is wrapped in its own decoration, so the list keeps
//! one child per row, as the scroll reveal, the pinned header, and the drags
//! count them.

use crate::config::{Theme, TreeHierarchy, mix};
use gpui::{prelude::*, *};

/// The guide's distance from the list's edge: the middle of the host's mark,
/// which starts where a header's content does.
const GUIDE_X: f32 = 24.;
/// How far the guide moves a host's rows in, clear of the line.
const GUIDE_STEP: f32 = 22.;
/// A block's margin from the sidebar's edges and its corners.
const BLOCK_MARGIN: f32 = 4.;
const BLOCK_RADIUS: f32 = 12.;

/// Where an item sits in its host: the header (0), a space (1), or an agent
/// or tab (2), and whether it opens or closes the host's run of rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sidebar) struct Place {
    pub(in crate::sidebar) level: u8,
    pub(in crate::sidebar) first: bool,
    pub(in crate::sidebar) last: bool,
}

/// How far a space row's content steps in under its header, beyond what
/// its item adds.
pub(in crate::sidebar) fn space_nest(hierarchy: TreeHierarchy) -> f32 {
    match hierarchy {
        TreeHierarchy::Indent => 18.,
        TreeHierarchy::Block => 10.,
        TreeHierarchy::Guide => 0.,
    }
}

/// How much further an agent's or tab's row steps in than its space's.
pub(in crate::sidebar) fn agent_step(hierarchy: TreeHierarchy) -> f32 {
    match hierarchy {
        TreeHierarchy::Indent | TreeHierarchy::Guide => 22.,
        TreeHierarchy::Block => 20.,
    }
}

/// The room between one host's last row and the next host's header.
fn host_gap(hierarchy: TreeHierarchy) -> f32 {
    match hierarchy {
        TreeHierarchy::Indent | TreeHierarchy::Guide => 18.,
        TreeHierarchy::Block => 10.,
    }
}

/// A host block's fill: a step from the sidebar towards the active colour.
pub(in crate::sidebar) fn block_fill(theme: &Theme) -> u32 {
    mix(theme.sidebar_background(), theme.active, 32)
}

/// The margin and corners a pinned header takes to sit on its block.
pub(in crate::sidebar) fn pinned_block(row: Stateful<Div>, theme: &Theme) -> Stateful<Div> {
    row.mx(px(BLOCK_MARGIN))
        .rounded_t(px(BLOCK_RADIUS))
        .bg(rgb(block_fill(theme)))
}

/// `element` as one item of the list, decorated for `place`. `key` names its
/// decoration for probes. The gap after a host's last row is padding under
/// it rather than over the next header, so a header's bounds stay its own.
pub(in crate::sidebar) fn item(
    element: impl IntoElement,
    place: Place,
    hierarchy: TreeHierarchy,
    key: &str,
    theme: &Theme,
) -> Div {
    let gap = if place.last { host_gap(hierarchy) } else { 0. };
    let item = div().w_full().flex_none().pb(px(gap));
    match hierarchy {
        TreeHierarchy::Indent => item.child(element),
        TreeHierarchy::Guide => {
            let line = (place.level > 0).then(|| {
                div()
                    .debug_selector(|| format!("tree-guide-{key}"))
                    .absolute()
                    .left(px(GUIDE_X))
                    .top_0()
                    .bottom_0()
                    .w(px(1.))
                    .bg(rgb(theme.active))
            });
            item.child(
                div()
                    .relative()
                    .w_full()
                    .when(place.level > 0, |inner| inner.pl(px(GUIDE_STEP)))
                    .child(element)
                    .children(line),
            )
        }
        TreeHierarchy::Block => item.px(px(BLOCK_MARGIN)).child(
            div()
                .debug_selector(|| format!("tree-block-{key}"))
                .w_full()
                .bg(rgb(block_fill(theme)))
                .when(place.first, |block| block.rounded_t(px(BLOCK_RADIUS)))
                .when(place.last, |block| block.rounded_b(px(BLOCK_RADIUS)))
                .child(element),
        ),
    }
}
