//! The workspace popover: a header naming the workspace, a fixed grid of
//! large action tiles, plain rows for the rarer actions, the destructive
//! action set apart, and the pull request section below.

use crate::{
    HerdrWindow,
    design::{Header, ItemState, Leading},
    menu::{WorkspaceAction, WorkspaceMenuAction},
};
use gpui::{prelude::*, *};

/// The grid's fixed slots, row by row: working on the checkout, then moving
/// or leaving it. A slot lists the actions it can show, first available
/// first, and the action it shows grayed when none is.
const SLOTS: [(&[WorkspaceMenuAction], WorkspaceMenuAction, &str); 6] = {
    use WorkspaceAction::*;
    use WorkspaceMenuAction::*;
    [
        (&[Dialog(NewWorktree)], Dialog(NewWorktree), "New worktree"),
        (&[FanOut], FanOut, "Fan out prompt..."),
        (&[Dialog(Rename)], Dialog(Rename), "Rename"),
        (&[GoToTeleported, Teleport], Teleport, "Teleport..."),
        (
            &[ClearTeleported, TeleportBack],
            TeleportBack,
            "Teleport back",
        ),
        (&[Dialog(Close)], Dialog(Close), "Close"),
    ]
};

const COLUMNS: usize = 3;

/// A tile's corners and icon: rounder and smaller than a plain menu's
/// button, so the grid reads as one quiet block.
const TILE_RADIUS: f32 = 10.;
const TILE_ICON: f32 = 18.;

/// One grid cell: the action it runs, its menu label, and whether this
/// workspace can take it now. A grayed tile keeps the grid's shape stable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::menu) struct Tile {
    pub(in crate::menu) action: WorkspaceMenuAction,
    pub(in crate::menu) label: &'static str,
    pub(in crate::menu) enabled: bool,
}

impl Tile {
    /// The tile's caption: the menu label, shortened where the tile's icon
    /// and position already say the rest.
    fn caption(self) -> &'static str {
        match self.action {
            WorkspaceMenuAction::FanOut if self.label.starts_with("Fan-out") => "Comparison",
            WorkspaceMenuAction::FanOut => "Fan out",
            WorkspaceMenuAction::GoToTeleported => "Go to copy",
            WorkspaceMenuAction::ClearTeleported => "Clear mark",
            _ => self.label.trim_end_matches("..."),
        }
    }
}

/// Lays `items` into the grid. Items no slot takes are returned in order,
/// for the rows beneath it.
pub(in crate::menu) fn layout(
    items: &[(WorkspaceMenuAction, &'static str)],
) -> ([Tile; 6], Vec<(WorkspaceMenuAction, &'static str)>) {
    let tiles = SLOTS.map(|(choices, fallback, label)| {
        choices
            .iter()
            .find_map(|choice| items.iter().find(|(action, _)| action == choice))
            .map_or(
                Tile {
                    action: fallback,
                    label,
                    enabled: false,
                },
                |&(action, label)| Tile {
                    action,
                    label,
                    enabled: true,
                },
            )
    });
    let rest = items
        .iter()
        .filter(|(action, _)| {
            !tiles
                .iter()
                .any(|tile| tile.enabled && tile.action == *action)
        })
        .copied()
        .collect();
    (tiles, rest)
}

impl HerdrWindow {
    pub(in crate::menu) fn render_workspace_popover(
        &self,
        pr_width: Pixels,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut panel = div().flex().flex_col();
        let look = self.menu_look();
        if let Some(target) = &self.menu.target {
            let branch = target
                .branch
                .as_deref()
                .filter(|branch| !branch.trim().is_empty());
            panel =
                panel.child(look.header(
                    Header {
                        selector: "workspace-menu-header",
                        icon: "icons/folder.svg",
                        title: ("workspace-menu-name", &target.label),
                        sub: branch.map(|branch| {
                            ("icons/git-branch.svg", "workspace-menu-branch", branch)
                        }),
                    },
                ));
        }
        let (tiles, rest) = layout(&self.workspace_items());
        let mut grid = div()
            .debug_selector(|| "workspace-menu-tiles".into())
            .flex()
            .flex_col()
            .gap(px(6.))
            .p(px(2.));
        for row in tiles.chunks(COLUMNS) {
            grid = grid.child(
                div()
                    .flex()
                    .gap(px(6.))
                    .children(row.iter().map(|tile| self.render_workspace_tile(*tile, cx))),
            );
        }
        panel = panel.child(grid);
        let delete = WorkspaceMenuAction::Dialog(WorkspaceAction::DeleteWorktree);
        let (rows, destructive): (Vec<_>, Vec<_>) =
            rest.into_iter().partition(|(action, _)| *action != delete);
        if !rows.is_empty() {
            panel = panel.child(div().h(px(6.)));
        }
        for (action, label) in rows {
            panel = panel.child(self.render_workspace_row(action, label, cx));
        }
        for (action, label) in destructive {
            panel = panel.child(self.render_workspace_danger(action, label, cx));
        }
        if self.pr_profile().is_some() {
            panel = panel.child(self.render_workspace_pr(pr_width, cx));
        }
        panel
    }

    /// Hover selects, as the arrow keys do, so both highlight the same item.
    fn workspace_hover(
        action: WorkspaceMenuAction,
        cx: &mut Context<Self>,
    ) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
        cx.listener(move |this, hovered: &bool, _, cx| {
            if *hovered {
                this.menu.workspace_selected = Some(action);
            } else if this.menu.workspace_selected == Some(action) {
                this.menu.workspace_selected = None;
            }
            cx.notify();
        })
    }

    fn render_workspace_tile(&self, tile: Tile, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = &self.theme;
        let look = self.menu_look();
        let label = tile.label;
        let selected = tile.enabled && self.menu.workspace_selected == Some(tile.action);
        let primary = tile.action == WorkspaceMenuAction::Dialog(WorkspaceAction::NewWorktree);
        // The primary tile speaks in the accent; the rest stay quiet until
        // selected.
        let (ink, icon_ink, fill) = if primary && tile.enabled {
            let accent = look.accent();
            let wash = if selected { 0.22 } else { 0.13 };
            (accent, accent, Rgba { a: wash, ..accent })
        } else if selected {
            let ink = rgb(theme.foreground);
            (ink, ink, rgb(theme.active))
        } else {
            let fill = Rgba {
                a: 0.45,
                ..rgb(theme.active)
            };
            (rgb(theme.subtext()), look.muted(), fill)
        };
        div()
            .id(label)
            .debug_selector(move || format!("workspace-menu-{label}"))
            .flex_1()
            .min_w_0()
            .min_h(px(64.))
            .px(px(4.))
            .py(px(10.))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .rounded(px(TILE_RADIUS))
            .bg(fill)
            .text_color(ink)
            .text_size(px(look.scale().secondary))
            .when_some(tile.action.icon(), |tile_div, icon| {
                tile_div.child(
                    svg()
                        .path(icon)
                        .size(px(TILE_ICON))
                        .flex_none()
                        .text_color(icon_ink)
                        .debug_selector(move || format!("workspace-menu-icon-{label}")),
                )
            })
            .child(
                div()
                    .debug_selector(move || format!("workspace-menu-label-{label}"))
                    .max_w_full()
                    .truncate()
                    .child(tile.caption()),
            )
            .map(|tile_div| {
                if !tile.enabled {
                    return tile_div.opacity(0.4);
                }
                let action = tile.action;
                tile_div
                    .cursor_pointer()
                    .on_hover(Self::workspace_hover(action, cx))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.activate_workspace_menu(action, window, cx);
                    }))
            })
    }

    /// A rarer action, as a menu row. Its icon leads, in the same column as
    /// the destructive row's, so the rows read as one list under the tiles.
    fn render_workspace_row(
        &self,
        action: WorkspaceMenuAction,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        self.workspace_menu_row(action, label, false, cx)
    }

    /// The destructive action: red, filled only while selected, so it never
    /// reads as one more of the tiles.
    fn render_workspace_danger(
        &self,
        action: WorkspaceMenuAction,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        self.workspace_menu_row(action, label, true, cx)
    }

    fn workspace_menu_row(
        &self,
        action: WorkspaceMenuAction,
        label: &'static str,
        danger: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let look = self.menu_look();
        let state = ItemState {
            selected: Some(action) == self.menu.workspace_selected,
            danger,
            ..ItemState::default()
        };
        let leading = action.icon().map_or(Leading::Empty, |icon| {
            Leading::Icon(icon, format!("workspace-menu-icon-{label}"))
        });
        look.row(label, format!("workspace-menu-{label}"), leading, state)
            .child(look.label(format!("workspace-menu-label-{label}"), label))
            .on_hover(Self::workspace_hover(action, cx))
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                this.activate_workspace_menu(action, window, cx);
            }))
    }
}
