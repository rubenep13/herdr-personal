//! Painting the new worktree dialog: the shared search, the tab strip, and the
//! listings behind it. Rows are drawn from the prepared filter, never from a
//! query made while rendering.

use super::{
    Page, WorkspaceAction,
    worktree_source::{Row, Tab},
};
use crate::{
    HerdrWindow,
    design::{ItemState, Leading},
    repo_items::{Kind, Origin},
};
use gpui::{prelude::*, *};

impl HerdrWindow {
    /// The search every tab shares, then the tab strip. Each listed tab counts
    /// what the search kept in it, so a match elsewhere is visible from any
    /// tab. The GitHub tabs are inert until an account is connected, because
    /// neither listing can be fetched without one, and look it.
    pub(super) fn render_worktree_tabs(&self, cx: &mut Context<Self>) -> Div {
        let look = self.menu_look();
        let connected = self.menu.github.connected();
        let Some(source) = &self.menu.worktree else {
            return div();
        };
        let busy = source.busy();
        let mut strip = look.tabs("worktree-tabs");
        for tab in Tab::ALL {
            let label = tab.label();
            let enabled = connected || tab.kind().is_none();
            let selected = tab == source.tab;
            // A tab that cannot be opened fades and takes no click, so it
            // never reads as one more tab to press.
            let inert = !selected && (!enabled || busy);
            let hits = source.hits(tab).filter(|_| enabled);
            strip = strip.child(
                look.tab(
                    label,
                    format!("worktree-tab-{label}"),
                    "worktree-tab-underline",
                    selected,
                )
                .gap(look.at(5.))
                .child(label)
                .when_some(hits, |button, hits| {
                    button.child(
                        div()
                            .debug_selector(move || format!("worktree-tab-count-{label}"))
                            .text_size(px(look.scale().detail))
                            .text_color(look.muted())
                            .child(hits.to_string()),
                    )
                })
                .when(inert, |button| {
                    button.opacity(0.4).cursor(CursorStyle::OperationNotAllowed)
                })
                .when(enabled && !busy, |button| {
                    button.on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.select_worktree_tab(tab, window, cx);
                    }))
                }),
            );
        }
        div()
            .flex_none()
            .child(
                div()
                    .debug_selector(|| "worktree-search".into())
                    .child(look.query("worktree-search-icon", source.search.clone())),
            )
            .child(strip)
    }

    /// The open listing: how much of it the search kept, the rows themselves,
    /// and whatever its source last had to say.
    pub(super) fn render_worktree_items(&self, cx: &mut Context<Self>) -> Div {
        let theme = &self.theme;
        let font = &self.config.ui;
        let look = self.menu_look();
        let Some(source) = &self.menu.worktree else {
            return div();
        };
        let (shown, total) = source.counts();
        let rows = source.filtered.len();
        let (noun, loading, message, empty) = match source.tab {
            Tab::New => return div(),
            Tab::Existing => (
                "checkouts not open in Herdr",
                source.checkouts.request.is_some(),
                source.checkouts.message.as_ref(),
                if total > 0 {
                    "No matching checkouts."
                } else {
                    "Every checkout of this repository is already open."
                },
            ),
            Tab::Branches => (
                "local branches without a checkout",
                source.branches.loading,
                source.branches.message.as_ref(),
                if total > 0 {
                    "No matching branches."
                } else {
                    "Every branch already has a checkout."
                },
            ),
            Tab::Items(kind) => (
                match kind {
                    Kind::PullRequest => "open pull requests",
                    Kind::Issue => "open issues",
                },
                source.lookup.loading,
                source.lookup.message.as_ref(),
                kind.empty_label(),
            ),
        };
        let message = self.menu.error.as_ref().or(message);
        let status = if let Some(pending) = &source.pending {
            // Dismissing only closes the panel; the daemon keeps queued work,
            // as the branch tab's own waiting note says.
            if pending.opens() {
                format!(
                    "Opening {}. Dismissing does not cancel it.",
                    pending.label()
                )
            } else {
                format!(
                    "Creating a checkout for {}. Dismissing does not cancel it.",
                    pending.label()
                )
            }
        } else if let Some(error) = message {
            error.clone()
        } else if loading {
            "Loading...".to_owned()
        } else {
            match source.tab {
                Tab::Existing => "Opening adds the checkout to Herdr; nothing is created.".into(),
                Tab::Branches => "The checkout uses the local branch as it is.".into(),
                _ => source
                    .lookup
                    .origin
                    .as_ref()
                    .map(Origin::slug)
                    .unwrap_or_default(),
            }
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .debug_selector(|| "worktree-count".into())
                    .flex_none()
                    .px(px(16.))
                    .pt(px(8.))
                    .text_color(rgb(theme.muted))
                    .child(format!("{shown} of {total} {noun}")),
            )
            .when(rows == 0, |panel| {
                panel.child(
                    div()
                        .debug_selector(|| "worktree-empty".into())
                        .flex_1()
                        .px(px(16.))
                        .py(px(12.))
                        .text_color(rgb(theme.muted))
                        .child(if loading { "Loading..." } else { empty }),
                )
            })
            .when(rows > 0, |panel| {
                panel.child(
                    uniform_list(
                        "worktree-items",
                        rows,
                        cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                            range.map(|row| this.render_worktree_row(row, cx)).collect()
                        }),
                    )
                    .track_scroll(&source.scroll)
                    .flex_1()
                    .min_h_0()
                    .px(look.at(6.))
                    .py(look.at(4.)),
                )
            })
            .child(
                div()
                    .id("worktree-status")
                    .debug_selector(|| "worktree-status".into())
                    .flex_none()
                    .h(px(font.line_height() * 2. + 16.))
                    .overflow_y_scroll()
                    .px(px(16.))
                    .py(px(8.))
                    .border_t_1()
                    .border_color(rgb(theme.active))
                    .text_color(rgb(theme.muted))
                    .when(message.is_some(), |status| {
                        status.text_color(super::danger(theme))
                    })
                    .child(
                        div()
                            .when(message.is_some(), |text| {
                                text.debug_selector(|| "dialog-error".into())
                            })
                            .child(status),
                    ),
            )
    }

    fn render_worktree_row(&self, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(source) = &self.menu.worktree else {
            return div().into_any_element();
        };
        let Some(listed) = source.row(row) else {
            return div().into_any_element();
        };
        // A row is a title line with an optional muted tag, and a detail line
        // when there is more to say than the title. A branch is only its name.
        let (number, title, tag, detail) = match listed {
            Row::Checkout(entry) => (
                None,
                entry.branch.clone().unwrap_or_else(|| entry.label.clone()),
                entry.is_detached.then_some("detached"),
                Some(entry.path.clone()),
            ),
            Row::Branch(branch) => (None, branch.name.clone(), None, None),
            Row::Item(item) => (
                Some(format!("#{}", item.number)),
                item.title.clone(),
                item.draft.then_some("draft"),
                // A fork's head branch has no ref on `origin`, so the row says
                // why it cannot be picked rather than failing once it is.
                Some(if item.fork_owner.is_some() {
                    "from a fork - check out manually".to_owned()
                } else {
                    let branch = item.branch();
                    match item.author.is_empty() {
                        true => branch,
                        false => format!("{} - {branch}", item.author),
                    }
                }),
            ),
        };
        let look = self.menu_look();
        let state = ItemState {
            selected: row == source.selected,
            ..ItemState::default()
        };
        let icon = match listed {
            Row::Checkout(_) => "icons/folder.svg",
            Row::Branch(_) => "icons/git-branch.svg",
            Row::Item(item) if item.kind == Kind::Issue => "icons/note.svg",
            Row::Item(_) => "icons/git-branch.svg",
        };
        let leading = Leading::Icon(icon, format!("worktree-row-{row}-icon"));
        let title = div()
            .flex()
            .gap(look.at(6.))
            .min_w_0()
            .when_some(number, |line, number| {
                line.child(div().flex_none().text_color(look.muted()).child(number))
            })
            .child(div().flex_1().min_w_0().truncate().child(title))
            .when_some(tag, |line, tag| {
                line.child(div().flex_none().text_color(look.muted()).child(tag))
            });
        // A row with a detail line is a list's two-line row; a branch is a
        // menu's one-line row.
        let frame = match detail {
            Some(detail) => look
                .list_row(row, format!("worktree-row-{row}"), state)
                .child(look.leading(leading, state))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(title)
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(px(look.scale().detail))
                                .text_color(look.muted())
                                .child(detail),
                        ),
                ),
            None => look
                .row(row, format!("worktree-row-{row}"), leading, state)
                .child(div().flex_1().min_w_0().child(title)),
        };
        frame
            .w_full()
            .on_hover(cx.listener(move |this, hovered, _, cx| {
                if *hovered && let Some(source) = &mut this.menu.worktree {
                    source.selected = row;
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.pick_worktree_row(row, cx);
            }))
            .into_any_element()
    }

    /// Whether the shared search field is mid-composition, in which case the
    /// platform owns every key until the composition commits.
    pub(super) fn worktree_source_composing(&self, cx: &Context<Self>) -> bool {
        self.menu
            .worktree
            .as_ref()
            .is_some_and(|source| source.search.read(cx).is_composing())
    }

    /// Keys the new worktree dialog's listings own. Reports whether the key was
    /// consumed, so the branch form keeps its existing handling untouched.
    pub(super) fn worktree_source_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.menu.page != Some(Page::Dialog(WorkspaceAction::NewWorktree)) {
            return false;
        }
        if self
            .menu
            .input
            .as_ref()
            .is_some_and(|input| input.marked.is_some())
        {
            return false;
        }
        let key = event.keystroke.key.as_str();
        if key == "tab" {
            let forward = !event.keystroke.modifiers.shift;
            let current = self
                .menu
                .worktree
                .as_ref()
                .map_or(Tab::New, |source| source.tab);
            let connected = self.menu.github.connected();
            let tabs: Vec<Tab> = Tab::ALL
                .into_iter()
                .filter(|tab| tab.kind().is_none() || connected)
                .collect();
            if let Some(index) = tabs.iter().position(|tab| *tab == current) {
                let next = if forward {
                    (index + 1) % tabs.len()
                } else {
                    (index + tabs.len() - 1) % tabs.len()
                };
                self.select_worktree_tab(tabs[next], window, cx);
            }
            return true;
        }
        let search_focused = self.worktree_search_focused(window, cx);
        let Some(source) = &mut self.menu.worktree else {
            return false;
        };
        if source.tab == Tab::New {
            // From the shared search, the branch form has no rows to move
            // through and nothing for Enter to pick.
            return search_focused && matches!(key, "up" | "down" | "enter");
        }
        let rows = source.filtered.len();
        match key {
            "up" | "down" if rows > 0 => {
                source.selected = match key {
                    "up" => (source.selected + rows - 1) % rows,
                    _ => (source.selected + 1) % rows,
                };
                let selected = source.selected;
                source.scroll.scroll_to_item(selected, ScrollStrategy::Top);
                cx.notify();
                true
            }
            "up" | "down" => true,
            "enter" => {
                let selected = source.selected;
                self.pick_worktree_row(selected, cx);
                true
            }
            _ => false,
        }
    }
}
