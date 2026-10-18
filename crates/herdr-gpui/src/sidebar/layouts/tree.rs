//! The Tree layout's own rows: a folder and its branch for each space, and
//! for each agent a tile with its mark, the tab it runs in, its kind, and a
//! state badge. Text reads in the interface face at the main window's type
//! scale and shrinks with an ellipsis rather than by a measured glyph budget,
//! since that face is proportional. Every colour comes from the theme, except
//! an agent's mark, which keeps its brand's.

use super::{
    super::{
        cell::{AgentRow, Fold, RowContext, RowLayout, RowState, WorkspaceRow},
        label_text,
        layout::ROW_GROUP,
        row::removing_dot,
        tokens::{TextRole, TokenKind},
    },
    parts,
};
use crate::{
    config::{AgentDetails, StatusIcons, Theme, mix},
    design::{AGENT_DETAILS_HEIGHT, AGENT_ROW_HEIGHT, ROW_HEIGHT, TypeScale},
    icons::AgentIcon,
    sidebar::Indicators,
};
use gpui::{prelude::*, *};
use herdr_client::protocol::AgentStatus;

pub(in super::super) struct Tree;

/// An agent's or tab's tile and the mark inside it.
const TILE: f32 = 24.;
const TILE_RADIUS: f32 = 7.;
const MARK: f32 = 15.;
const FOLDER: f32 = 16.;
const BRANCH_MARK: f32 = 11.;
const BADGE: f32 = 14.;
/// The fold and menu buttons at a space row's end.
const CONTROL: f32 = 20.;
/// Space between a row's leading icon and its text.
const LEAD_GAP: f32 = 10.;

/// The colour of `status`'s badge, inked to read on the theme's chrome.
pub(crate) fn badge_color(status: AgentStatus, indicators: Indicators, theme: &Theme) -> u32 {
    theme.ink(indicators.color(status))
}

/// The colour an agent's mark draws in on its tile: its brand's, else the
/// theme's text, inked either way so it reads on a light theme too.
pub(crate) fn logo_color(icon: AgentIcon, theme: &Theme) -> u32 {
    theme.ink(icon.brand().unwrap_or(theme.foreground))
}

/// The badge for `status`: Herdr's outline marks, with a warning for an
/// agent that waits on the user. The working arc holds still; a spinner
/// would redraw the whole sidebar every frame while any agent works.
fn badge_path(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Blocked => "icons/status-badge-blocked.svg",
        status => crate::icons::status_icon(StatusIcons::Rail, status),
    }
}

/// An agent's state badge in `status`'s colour, as the agent row ends with it.
pub(crate) fn badge(
    selector: String,
    status: AgentStatus,
    indicators: Indicators,
    theme: &Theme,
) -> Div {
    div()
        .debug_selector(move || selector)
        .size(px(BADGE))
        .flex_none()
        .child(parts::icon(
            badge_path(status),
            BADGE,
            badge_color(status, indicators, theme),
        ))
}

/// A row's frame: its height, the content inset by the nesting, and the
/// selection and host marks the look paints under it.
fn frame(key: &str, height: f32, state: RowState, cx: &RowContext<'_>) -> Div {
    let look = cx.look;
    div()
        .debug_selector(|| format!("row-{key}"))
        .relative()
        .w_full()
        .flex_none()
        .h(px(height))
        .pl(px(look.content_x() + cx.nest))
        .pr(px(look.content_x()))
        .flex()
        .items_center()
        .gap(px(LEAD_GAP))
        .cursor_pointer()
        .map(|row| cx.mark.apply(row, key, &look))
        .map(|row| look.mark(row, key, state, cx.theme))
}

/// One line of text that ends in an ellipsis when it does not fit.
fn text(selector: String, value: &str, size: f32) -> Div {
    div()
        .debug_selector(move || selector)
        .min_w_0()
        .truncate()
        .text_size(px(size))
        .line_height(px((size * 1.3).round()))
        .child(label_text(value))
}

/// A small label tinted with `color` over a wash of it.
pub(crate) fn pill(selector: String, value: &str, color: u32, size: f32) -> Div {
    div()
        .debug_selector(move || selector)
        .flex_none()
        .px(px(6.))
        .rounded(px(4.))
        .text_size(px(size))
        .line_height(px((size * 1.45).round()))
        .bg(parts::wash(color, 0x26))
        .text_color(rgb(color))
        .child(label_text(value))
}

/// A square button at a row's end that shows only while the row, a member of
/// hover `group`, is pointed at, or while `shown`, so a row at rest stays
/// quiet.
pub(in crate::sidebar) fn control(
    id: ElementId,
    selector: String,
    (shown, group): (bool, &'static str),
    theme: &Theme,
) -> Stateful<Div> {
    let hover = mix(theme.active, theme.foreground, 12);
    div()
        .id(id)
        .debug_selector(move || selector)
        .size(px(CONTROL))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(5.))
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(hover)))
        .when(!shown, |control| {
            control.hidden().group_hover(group, |style| style.flex())
        })
}

fn fold_button(fold: Fold, id: &str, shown: bool, theme: &Theme) -> Stateful<Div> {
    let Fold {
        id: element,
        collapsed,
        toggle,
        ..
    } = fold;
    let chevron = if collapsed {
        "icons/chevron-right.svg"
    } else {
        "icons/chevron-down.svg"
    };
    control(
        element,
        format!("space-fold-{id}"),
        (shown, ROW_GROUP),
        theme,
    )
    .child(parts::icon(chevron, 14., theme.muted))
    .on_click(move |event, window, cx| {
        cx.stop_propagation();
        toggle(event, window, cx);
    })
}

/// An agent's or tab's tile: `mark` drawn in `color` on the active colour,
/// a step lighter on the selected row, whose own fill is that colour.
pub(crate) fn tile(
    selector: String,
    mark: &'static str,
    color: u32,
    selected: bool,
    theme: &Theme,
) -> Div {
    let fill = if selected {
        mix(theme.active, theme.foreground, 10)
    } else {
        theme.active
    };
    div()
        .debug_selector(move || selector)
        .size(px(TILE))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(TILE_RADIUS))
        .bg(rgb(fill))
        .child(parts::icon(mark, MARK, color))
}

/// The tab's name over a second line, the column every agent and tab row
/// shares between its tile and its end.
fn titles(
    (selector, name): (String, &str),
    selected: bool,
    second: Div,
    cx: &RowContext<'_>,
) -> Div {
    let theme = cx.theme;
    let scale = TypeScale::of(cx.font);
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(
            text(selector, name, scale.name)
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(if selected {
                    theme.foreground
                } else {
                    theme.subtext()
                })),
        )
        .child(second)
}

/// The text tokens Herdr's agent rows add beyond what the row already names:
/// its own `$tokens` and terminal titles, such as a model and a summary.
fn details(lines: &[Vec<super::super::tokens::ResolvedToken>]) -> Vec<&SharedString> {
    lines
        .iter()
        .flatten()
        .filter_map(|token| match &token.kind {
            TokenKind::Text(text, TextRole::Muted) => Some(text),
            _ => None,
        })
        .collect()
}

impl RowLayout for Tree {
    fn workspace(&self, row: WorkspaceRow<'_>, state: RowState, cx: &RowContext<'_>) -> Div {
        let theme = cx.theme;
        let scale = TypeScale::of(cx.font);
        let id = row.workspace.workspace_id.as_str();
        let (branch, upstream) = (row.branch(), row.upstream());
        let shown = state.selected || state.highlighted;
        let WorkspaceRow {
            label,
            fold,
            badge,
            removing,
            ports,
            attention,
            more,
            ..
        } = row;
        let lead = if removing {
            div()
                .w(px(FOLDER))
                .flex_none()
                .flex()
                .justify_center()
                .child(removing_dot("worktree-removing", theme))
        } else {
            div()
                .debug_selector(move || format!("space-icon-{id}"))
                .flex_none()
                .child(parts::icon("icons/folder.svg", FOLDER, theme.muted))
        };
        // The name keeps its width; the branch gives way first, to nothing,
        // before the name is cut.
        let name = text(format!("name-{label}"), label, scale.name)
            .flex_shrink_1()
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(if state.selected {
                theme.foreground
            } else {
                theme.subtext()
            }));
        let glyph = parts::glyph_at(cx.font, scale.secondary);
        let branch = branch.map(|branch| {
            div()
                .debug_selector(move || format!("space-branch-{id}"))
                .flex()
                .items_center()
                .gap(px(4.))
                .min_w_0()
                .flex_shrink(1000.)
                .overflow_hidden()
                .text_size(px(scale.secondary))
                .text_color(rgb(theme.muted))
                .child(parts::icon(
                    "icons/git-branch.svg",
                    BRANCH_MARK,
                    theme.muted,
                ))
                .child(div().min_w_0().truncate().child(label_text(branch)))
                .when_some(upstream, |line, upstream| {
                    line.child(upstream.element(id, glyph, theme))
                })
        });
        let marks = badge.map(|badge| {
            let size = (scale.secondary + 2.).round();
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(scale.secondary))
                .when_some(badge.pr.as_ref(), |marks, pr| {
                    marks.child(parts::pr_number(id, pr, BRANCH_MARK, glyph).1)
                })
                .when(badge.dirty, |marks| {
                    marks.child(parts::dirty(id, size, theme))
                })
                .when(badge.noted, |marks| {
                    marks.child(crate::icons::note(theme, size))
                })
                .when(badge.teleported, |marks| {
                    marks.child(parts::teleported(id, size, theme))
                })
        });
        let blocked = badge_color(AgentStatus::Blocked, cx.indicators, theme);
        let end = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .children(marks)
            .children(ports)
            .when(attention > 0, |end| {
                end.child(pill(
                    format!("attention-{id}"),
                    &attention.to_string(),
                    blocked,
                    scale.detail - 1.,
                ))
            })
            .when_some(more, |end, more| {
                end.child(
                    control(
                        SharedString::from(format!("space-more-{id}")).into(),
                        format!("space-more-{id}"),
                        (shown, ROW_GROUP),
                        theme,
                    )
                    .child(parts::icon("icons/more.svg", 14., theme.muted))
                    .on_click(move |event, window, cx| {
                        cx.stop_propagation();
                        more(event, window, cx);
                    }),
                )
            })
            .when_some(fold, |end, fold| {
                end.child(fold_button(fold, id, shown, theme))
            });
        frame(label, ROW_HEIGHT, state, cx)
            .child(lead)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(name)
                    .children(branch),
            )
            .child(end)
    }

    fn agent(&self, agent: AgentRow<'_>, state: RowState, cx: &RowContext<'_>) -> Div {
        let theme = cx.theme;
        let scale = TypeScale::of(cx.font);
        let id = agent.key.strip_prefix("agent-").unwrap_or(&agent.key);
        let tree = cx.look.tree;
        let details = match tree.details {
            AgentDetails::Herdr => details(&agent.lines),
            AgentDetails::Off => Vec::new(),
        };
        let state_text = agent.status_text.as_deref().filter(|_| tree.state_text);
        let color = badge_color(agent.status, cx.indicators, theme);
        let second = div()
            .flex()
            .items_center()
            .gap(px(7.))
            .min_w_0()
            .text_color(rgb(theme.muted))
            .children(
                agent
                    .kind
                    .map(|kind| text(format!("detail-{}", agent.key), kind, scale.secondary)),
            )
            .when_some(state_text, |line, state| {
                line.child(pill(
                    format!("agent-state-{id}"),
                    state,
                    color,
                    scale.detail - 1.,
                ))
            });
        let second =
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .child(second)
                .when(!details.is_empty(), |column| {
                    column.child(
                        div()
                            .debug_selector(move || format!("agent-details-{id}"))
                            .flex()
                            .items_center()
                            .gap(px(7.))
                            .min_w_0()
                            // The first, a model, gives way only after the rest.
                            .children(details.iter().enumerate().map(|(index, detail)| {
                                text(format!("agent-detail-{id}-{index}"), detail, scale.detail)
                                    .when(index == 0, |first| {
                                        first.text_color(rgb(theme.subtext()))
                                    })
                                    .when(index > 0, |rest| {
                                        rest.flex_shrink(1000.).text_color(rgb(theme.muted))
                                    })
                            })),
                    )
                });
        let height = if details.is_empty() {
            AGENT_ROW_HEIGHT
        } else {
            AGENT_DETAILS_HEIGHT
        };
        frame(&agent.key, height, state, cx)
            .child(tile(
                format!("agent-tile-{id}"),
                agent.icon.path(),
                logo_color(agent.icon, theme),
                state.selected,
                theme,
            ))
            .child(titles(
                (format!("name-{}", agent.key), agent.name),
                state.selected,
                second,
                cx,
            ))
            .child(badge(
                format!("agent-badge-{id}"),
                agent.status,
                cx.indicators,
                theme,
            ))
    }
}

/// A tab no agent runs in: the terminal's tile, its name, and "Terminal"
/// under it, with no state, since nothing reports one.
pub(in crate::sidebar) fn tab_row(
    (id, label): (&str, &str),
    state: RowState,
    cx: &RowContext<'_>,
) -> Div {
    let theme = cx.theme;
    let scale = TypeScale::of(cx.font);
    let second =
        text(format!("tab-kind-{id}"), "Terminal", scale.secondary).text_color(rgb(theme.muted));
    frame(&format!("tab-{id}"), AGENT_ROW_HEIGHT, state, cx)
        .child(tile(
            format!("tab-tile-{id}"),
            "icons/terminal.svg",
            theme.muted,
            state.selected,
            theme,
        ))
        .child(titles(
            (format!("tab-title-{id}"), label),
            state.selected,
            second,
            cx,
        ))
}
