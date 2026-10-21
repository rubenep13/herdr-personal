//! The checkpoints dialog of a workspace row: the checkout's snapshots, newest
//! first, each with what its turn changed, and a restore behind a
//! confirmation that says what restoring replaces.

use super::Page;
use crate::{
    HerdrWindow,
    checkpoint::{Checkout, Listing, age, host_for},
    design::{ButtonKind, Header, NoteTone},
};
use gpui::{prelude::*, *};
use herdr_client::protocol::AgentStatus;

impl HerdrWindow {
    /// The checkout the menu's workspace shows, when checkpoints can reach it.
    pub(crate) fn checkpoint_checkout(&self) -> Option<Checkout> {
        let target = self.menu.target.as_ref()?;
        let endpoint = self.endpoints.get(self.selected_endpoint)?;
        let host = host_for(&endpoint.connection.target, &self.live)?;
        Checkout::new(
            host,
            &target.worktree.as_ref()?.key,
            target.branch.as_deref(),
        )
    }

    pub(crate) fn open_checkpoints(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(checkout), Some(target)) = (self.checkpoint_checkout(), &self.menu.target) else {
            return;
        };
        let workspace = target.id.clone();
        self.checkpoints
            .open(checkout, target.label.clone(), workspace);
        self.menu.page = Some(Page::Checkpoints);
        self.menu.error = None;
        window.focus(&self.menu.focus, cx);
        cx.notify();
    }

    /// Closing the dialog forgets its list; a restore it started still runs.
    pub(crate) fn close_stale_checkpoints(&mut self) {
        if self.menu.page != Some(Page::Checkpoints) && self.checkpoints.view.is_some() {
            self.checkpoints.close();
        }
    }

    /// Whether an agent in the dialog's workspace is mid-turn, and so may
    /// keep changing the files a restore puts back.
    fn checkpoint_agent_working(&self) -> bool {
        let Some(view) = &self.checkpoints.view else {
            return false;
        };
        self.live.snapshot.as_ref().is_some_and(|snapshot| {
            snapshot.agents.iter().any(|agent| {
                agent.workspace_id == view.workspace_id
                    && matches!(
                        agent.agent_status,
                        AgentStatus::Working | AgentStatus::Blocked
                    )
            })
        })
    }

    pub(crate) fn checkpoints_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        window.prevent_default();
        let Some(view) = &mut self.checkpoints.view else {
            self.dismiss_menu(window, cx);
            return;
        };
        let count = match &view.listing {
            Listing::Ready(list) => list.len(),
            _ => 0,
        };
        match event.keystroke.key.as_str() {
            "escape" if view.confirming.is_some() && !view.restoring => view.confirming = None,
            "escape" => self.dismiss_menu(window, cx),
            "up" | "down" if view.confirming.is_none() && count > 0 => {
                let up = event.keystroke.key == "up";
                view.selected = Some(match view.selected {
                    None if up => count - 1,
                    None => 0,
                    Some(index) if up => (index + count - 1) % count,
                    Some(index) => (index + 1) % count,
                });
            }
            "enter" if view.confirming.is_some() => self.checkpoints.restore(),
            "enter" => {
                if let Some(checkpoint) = view
                    .selected_checkpoint()
                    .filter(|checkpoint| view.restorable(checkpoint))
                {
                    view.confirming = Some(checkpoint.id.clone());
                }
            }
            _ => return,
        }
        cx.notify();
    }

    pub(crate) fn render_checkpoints(&self, cx: &mut Context<Self>) -> Div {
        let Some(view) = &self.checkpoints.view else {
            return div();
        };
        let theme = &self.theme;
        let muted = rgb(theme.muted);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| {
                i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
            });
        let look = self.dialog_look();
        let text_size = px(look.settings().scale().name);
        let line = |text: String| {
            div()
                .min_w_0()
                .text_size(text_size)
                .text_color(rgb(theme.subtext()))
                .child(text)
        };
        let mut body = look.body("checkpoints-sheet");
        let confirming = view.confirming_checkpoint();
        if let Some(checkpoint) = confirming {
            body = body
                .child(
                    line(format!(
                        "Restore \u{201c}{}\u{201d} from {}?",
                        checkpoint.label,
                        age(checkpoint.created, now)
                    ))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(theme.foreground)),
                )
                .child(line(format!(
                    "Files in this checkout go back to how they were then, and {} moves back to the commit it was on. Files created since are removed; ignored files are left alone.",
                    view.checkout.branch
                )))
                .child(
                    line("The current state is saved as a checkpoint first, so this can be undone.".into())
                        .text_color(muted),
                );
            if self.checkpoint_agent_working() {
                body = body.child(look.note(
                    "checkpoints-agent-working".into(),
                    NoteTone::Warning,
                    "An agent in this workspace is still working and may keep changing files.",
                ));
            }
        } else {
            match &view.listing {
                Listing::Loading => {
                    body = body.child(line("Loading checkpoints...".into()).text_color(muted));
                }
                Listing::Failed(error) => {
                    body = body.child(look.error("checkpoints-error".into(), error.clone()));
                }
                Listing::Ready(list) if list.is_empty() => {
                    body = body.child(
                        line(if self.config.agent_checkpoints {
                            "No checkpoints yet. One is taken each time an agent here starts or finishes a turn.".into()
                        } else {
                            "No checkpoints. Checkpoint agent turns is off in Settings > General.".into()
                        })
                        .text_color(muted),
                    );
                }
                Listing::Ready(list) => {
                    body = body.child(self.render_checkpoint_timeline(list, now, cx));
                }
            }
        }
        if let Some(error) = &view.error {
            body = body.child(look.error("checkpoints-restore-error".into(), error.clone()));
        }
        // Back from a confirmation, else close: what Esc does too.
        let back = |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| match &mut this
            .checkpoints
            .view
        {
            Some(view) if view.confirming.is_some() && !view.restoring => {
                view.confirming = None;
                cx.notify();
            }
            _ => this.dismiss_menu(window, cx),
        };
        let mut footer = look
            .footer("checkpoints-footer")
            .hint(
                "Esc",
                if confirming.is_some() {
                    "cancel"
                } else {
                    "close"
                },
            )
            .button(
                look.button(
                    "checkpoints-cancel",
                    if confirming.is_some() {
                        "Cancel"
                    } else {
                        "Close"
                    },
                    ButtonKind::Normal,
                    true,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    back(this, window, cx);
                })),
            );
        if confirming.is_some() {
            let armed = !view.restoring;
            footer = footer.button(
                look.button(
                    "checkpoints-restore",
                    if armed { "Restore" } else { "Restoring..." },
                    ButtonKind::Danger,
                    armed,
                )
                .when(armed, |button| {
                    button.on_click(cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.checkpoints.restore();
                        cx.notify();
                    }))
                }),
            );
        }
        let place = format!("{} \u{b7} {}", view.title, view.checkout.branch);
        div()
            .flex()
            .flex_col()
            .child(
                look.header(
                    Header {
                        selector: "checkpoints-header",
                        icon: "icons/refresh.svg",
                        title: ("checkpoints-title", "Checkpoints"),
                        sub: Some(("icons/folder.svg", "checkpoints-place", &place)),
                        danger: false,
                    },
                    look.menu()
                        .close_button("checkpoints-dismiss")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            back(this, window, cx);
                        })),
                ),
            )
            .child(
                div()
                    .id("checkpoints-body")
                    .max_h(px(420.))
                    .overflow_y_scroll()
                    .child(body),
            )
            .child(footer)
    }
}

#[cfg(test)]
mod tests;
mod timeline;
