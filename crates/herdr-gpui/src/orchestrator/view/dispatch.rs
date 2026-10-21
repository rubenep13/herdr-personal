//! Starting an agent on an item, and the run actions that change things:
//! the dispatch dialog, the confirmations for stopping an agent, removing a
//! worktree, or deleting a bead, and the banner with each action's outcome.

use super::{OrchestratorView, look::agent_icon};
use crate::{
    design::{ButtonKind, ButtonSize, Header, NoteTone},
    orchestrator::{
        Action, DispatchRequest, Notice, SourceKey,
        actions::valid_model,
        prompt::{self, MAX_EXTRA},
    },
    pull_request::MergeMethod,
    search_input::{Frame, SearchInput},
    teleport::AgentKind,
};
use gpui::{prelude::*, *};

#[cfg(test)]
mod tests;

/// The dispatch dialog's choices.
pub(crate) struct Dialog {
    item: String,
    /// The prompt profile, by index into the snapshot's; `None` is built-in.
    profile: Option<usize>,
    kind: Option<AgentKind>,
    branch: Entity<SearchInput>,
    extra: Entity<SearchInput>,
    /// The model the agent is started with; its own default when empty.
    model: Entity<SearchInput>,
    /// Another host to start on, by endpoint; the tab's own when `None`.
    pub(super) host: Option<String>,
    problem: Option<&'static str>,
    /// A read-only review of a pull request rather than work on an issue.
    review: bool,
}

/// A destructive action waiting for the user to confirm it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Confirm {
    Stop {
        run: String,
        agent: String,
    },
    Remove {
        run: String,
        branch: String,
    },
    DeleteBead {
        source: SourceKey,
        id: String,
    },
    /// Merging the pull request and head commit shown when it was asked.
    Merge {
        method: MergeMethod,
        target: crate::pr_actions::Target,
    },
}

/// How a confirmation's verb is drawn: in the danger red when it stops,
/// removes or deletes something, in the accent for a merge.
pub(super) fn verb_kind(confirm: &Confirm) -> ButtonKind {
    match confirm {
        Confirm::Merge { .. } => ButtonKind::Primary,
        Confirm::Stop { .. } | Confirm::Remove { .. } | Confirm::DeleteBead { .. } => {
            ButtonKind::Danger
        }
    }
}

impl Confirm {
    fn text(&self) -> (String, String, &'static str) {
        match self {
            Self::Stop { agent, .. } => (
                format!("Stop {agent}?"),
                "Sends Esc, then Ctrl-C to the agent. The worktree is kept.".into(),
                "Stop",
            ),
            Self::Remove { branch, .. } if branch.is_empty() => (
                "Remove run?".into(),
                "Forgets this failed run. It has no worktree to remove.".into(),
                "Remove",
            ),
            Self::Remove { branch, .. } => (
                "Remove worktree?".into(),
                format!(
                    "Removes the checkout of {branch}, discarding uncommitted changes. Committed work stays on the branch."
                ),
                "Remove",
            ),
            Self::Merge { method, target } => (
                format!("{} #{}?", method.action(), target.number),
                format!(
                    "Merges pull request #{} at head commit {}; GitHub refuses if the branch moved since.",
                    target.number,
                    target.head().get(..12).unwrap_or(target.head()),
                ),
                "Merge",
            ),
            Self::DeleteBead { id, .. } => (
                format!("Delete {id} permanently?"),
                "Runs bd delete --force. This cannot be undone and removes it for everyone using this .beads."
                    .into(),
                "Delete",
            ),
        }
    }

    /// The worker action it confirms; a merge goes through `pr_actions`.
    fn action(self) -> Option<Action> {
        match self {
            Self::Stop { run, .. } => Some(Action::Stop { run }),
            Self::Remove { run, .. } => Some(Action::Remove { run }),
            Self::DeleteBead { source, id } => Some(Action::DeleteBead { source, id }),
            Self::Merge { .. } => None,
        }
    }
}

impl OrchestratorView {
    pub(crate) fn load_agents(
        &mut self,
        endpoint: Option<String>,
        target: herdr_client::ConnectTarget,
        cx: &mut Context<Self>,
    ) {
        if !self
            .service
            .as_ref()
            .is_some_and(|service| service.load_agents(endpoint, target))
        {
            self.report(crate::orchestrator::Error::Busy, cx);
        }
    }

    pub(super) fn select_agent_host(&mut self, cx: &mut Context<Self>) {
        if let Some(dialog) = &mut self.dialog {
            dialog.kind = None;
            self.snapshot.installed = None;
            self.snapshot.installed_error = None;
            cx.emit(super::Event::LoadAgents {
                endpoint: dialog.host.clone(),
            });
        }
    }

    pub(super) fn reconcile_agents(&mut self) {
        if let Some(dialog) = &mut self.dialog
            && dialog.host == self.snapshot.installed_endpoint
            && let Some(installed) = &self.snapshot.installed
            && !dialog.kind.is_some_and(|kind| installed.contains(&kind))
        {
            dialog.kind = installed.first().copied();
        }
    }

    /// Opens the dispatch dialog for the item with canonical key `item`.
    pub(super) fn open_dispatch(
        &mut self,
        item: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_dialog(item, false, window, cx);
    }

    /// Opens the dialog for a read-only review of the pull request `item`.
    pub(super) fn open_review(
        &mut self,
        item: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_dialog(item, true, window, cx);
    }

    fn open_dialog(
        &mut self,
        item: String,
        review: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(found) = self.item(&item) else {
            return;
        };
        let branch_name = match (&found.pull_request, review) {
            (Some(_), true) => crate::orchestrator::naming::review_branch(found),
            _ => crate::orchestrator::naming::branch(found),
        };
        let field = |placeholder: &str, text: &str, cx: &mut Context<Self>| {
            let (ui, theme) = (self.look.ui.clone(), self.look.theme.clone());
            cx.new(|cx| {
                let mut input = SearchInput::new(cx);
                input.set_placeholder(placeholder, cx);
                input.set_appearance(ui, theme, cx);
                // Each field wears Settings' frame, its ring in the accent
                // while it has the keyboard.
                input.set_frame(Frame::Field, cx);
                if !text.is_empty() {
                    input.set_text_selected(text, cx);
                }
                input
            })
        };
        let branch = field("Branch name", &branch_name, cx);
        let extra = field("Extra instructions, appended verbatim (optional)", "", cx);
        let model = field("Agent default", "", cx);
        let kind = self
            .snapshot
            .installed
            .as_ref()
            .and_then(|installed| installed.first().copied());
        let focus = extra.read(cx).focus.clone();
        self.dialog = Some(Dialog {
            item,
            profile: None,
            kind,
            branch,
            extra,
            model,
            host: None,
            problem: None,
            review,
        });
        self.select_agent_host(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    pub(super) fn close_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let open = self.dialog.take().is_some() | self.confirm.take().is_some();
        if open {
            window.focus(&self.focus, cx);
            cx.notify();
        }
        open
    }

    fn submit_dispatch(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        let Some(kind) = dialog.kind.filter(|kind| {
            dialog.host == self.snapshot.installed_endpoint
                && self
                    .snapshot
                    .installed
                    .as_ref()
                    .is_some_and(|agents| agents.contains(kind))
        }) else {
            dialog.problem = Some("Pick an agent installed on this host.");
            cx.notify();
            return;
        };
        let branch = dialog.branch.read(cx).text().trim().to_owned();
        if !prompt::valid_branch(&branch) {
            dialog.problem = Some("That branch name is not one Git accepts.");
            cx.notify();
            return;
        }
        let extra = dialog.extra.read(cx).text().to_owned();
        if extra.len() > MAX_EXTRA {
            dialog.problem = Some("Extra instructions are limited to 16 KiB.");
            cx.notify();
            return;
        }
        let model = dialog.model.read(cx).text().trim().to_owned();
        if !model.is_empty() && !valid_model(&model) {
            dialog.problem = Some("A model name has no spaces and does not start with a dash.");
            cx.notify();
            return;
        }
        let host = dialog.host.clone();
        let Some(item) = self
            .snapshot
            .items
            .iter()
            .find(|item| item.key.canonical() == dialog.item)
        else {
            return;
        };
        let profile = dialog
            .profile
            .and_then(|index| self.snapshot.profiles.get(index));
        let remote = self
            .snapshot
            .repo
            .as_ref()
            .and_then(|info| info.remote.as_ref())
            .map(|remote| remote.url.as_str());
        let text = match (&item.pull_request, dialog.review) {
            (Some(pr), true) => prompt::compose_review(item, pr, remote, profile, &extra),
            _ => prompt::compose(item, profile, &extra),
        };
        let request = DispatchRequest {
            item_key: dialog.item.clone(),
            kind,
            model: (!model.is_empty()).then_some(model),
            prompt: text,
            branch,
            workspace_id: self.request.workspace_id.clone(),
            base: None,
            elsewhere: None,
        };
        match host {
            // The window knows the other host and sets the request up for it.
            Some(endpoint) => cx.emit(super::Event::Dispatch {
                request: Box::new(request),
                endpoint,
            }),
            None => self.act(Action::Dispatch(Box::new(request))),
        }
        self.dialog = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Hands `action` to the worker, or says why it could not take it.
    pub(super) fn act(&mut self, action: Action) {
        let taken = self
            .service
            .as_ref()
            .is_some_and(|service| service.act(action));
        if !taken {
            self.notice = Some(Notice {
                outcome: Err(std::sync::Arc::new(crate::orchestrator::Error::Busy)),
            });
        }
    }

    /// Sends the message field's text to the newest run of the open item.
    pub(super) fn send_message(&mut self, cx: &mut Context<Self>) {
        let text = self.message.read(cx).text().trim().to_owned();
        let Some(detail) = &self.detail else {
            return;
        };
        let runs = super::rows::Runs::new(&self.snapshot.runs, &self.snapshot.sessions, &self.live);
        let Some(run) = runs
            .of_item(&detail.key)
            .first()
            .map(|row| runs.runs[row.run].id.clone())
        else {
            return;
        };
        if text.is_empty() {
            return;
        }
        self.act(Action::Send { run, text });
        self.message.update(cx, |input, cx| input.clear(cx));
    }

    pub(super) fn ask(&mut self, confirm: Confirm, cx: &mut Context<Self>) {
        self.confirm = Some(confirm);
        cx.notify();
    }

    /// The outcome of the newest action, as the dialogs' note or error, with
    /// a dismiss button.
    pub(super) fn render_notice(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let look = &self.look;
        let view = look.view();
        let notice = self.notice.as_ref()?;
        let shown = match &notice.outcome {
            Ok(text) => view.dialog().note(
                "orchestrator-notice-text".into(),
                NoteTone::Done,
                (*text).to_owned(),
            ),
            Err(error) => view
                .dialog()
                .error("orchestrator-notice-text".into(), error.to_string()),
        };
        Some(
            div()
                .id("orchestrator-notice")
                .flex_none()
                .mx(view.at(12.))
                .my(view.at(8.))
                .flex()
                .items_start()
                .gap(view.at(8.))
                .child(shown.flex_1())
                .child(
                    look.icon_button("orchestrator-notice-close", "icons/close.svg")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.notice = None;
                            cx.notify();
                        })),
                )
                .into_any_element(),
        )
    }

    /// The dialog or confirmation over the view, if one is open, on the
    /// dialogs' dimming.
    pub(super) fn render_overlay(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let card = match (&self.dialog, &self.confirm) {
            (Some(dialog), _) => self.render_dialog(dialog, cx),
            (None, Some(confirm)) => self.render_confirm(confirm, cx),
            (None, None) => return None,
        };
        let view = self.look.view();
        let backdrop = (self.look.theme.background << 8) | 0xb0;
        Some(
            div()
                .id("orchestrator-overlay")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(rgba(backdrop))
                .flex()
                .items_center()
                .justify_center()
                .p(view.at(16.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                // A short window or large text scrolls the card rather than
                // pushing its fields and buttons out of reach.
                .child(
                    view.menu()
                        .panel(div())
                        .id("orchestrator-overlay-card")
                        .debug_selector(|| "orchestrator-overlay-card".into())
                        .max_h_full()
                        .max_w_full()
                        .overflow_y_scroll()
                        .child(card),
                )
                .into_any_element(),
        )
    }

    fn render_confirm(&self, confirm: &Confirm, cx: &mut Context<Self>) -> AnyElement {
        let look = &self.look;
        let view = look.view();
        let dialog = view.dialog();
        let (title, body, verb) = confirm.text();
        let kind = verb_kind(confirm);
        let confirm = confirm.clone();
        let close = view
            .menu()
            .close_button("orchestrator-confirm-close")
            .on_click(cx.listener(|this, _, window, cx| {
                this.close_overlays(window, cx);
            }));
        let verb_id = format!("orchestrator-confirm-{verb}");
        div()
            .w(view.at(CONFIRM_WIDTH))
            .flex()
            .flex_col()
            .child(dialog.header(
                Header {
                    selector: "orchestrator-confirm-header",
                    icon: "icons/alert.svg",
                    title: ("orchestrator-confirm-title", &title),
                    sub: None,
                    danger: kind == ButtonKind::Danger,
                },
                close,
            ))
            .child(
                dialog
                    .body("orchestrator-confirm-body")
                    .child(dialog.lead("orchestrator-confirm-lead", body)),
            )
            .child(
                dialog
                    .footer("orchestrator-confirm-footer")
                    .hint("Esc", "cancel")
                    .button(
                        dialog
                            .button(
                                "orchestrator-confirm-cancel",
                                "Cancel",
                                ButtonKind::Normal,
                                true,
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_overlays(window, cx);
                            })),
                    )
                    .button(
                        view.settings()
                            .button(
                                SharedString::from(verb_id.clone()),
                                verb_id,
                                verb,
                                kind,
                                ButtonSize::Regular,
                                true,
                            )
                            .on_click(cx.listener(move |this, _, window, cx| {
                                match confirm.clone() {
                                    Confirm::Merge { method, target } => {
                                        this.merge(method, &target, cx)
                                    }
                                    confirm => {
                                        if let Some(action) = confirm.action() {
                                            this.act(action);
                                        }
                                    }
                                }
                                this.close_overlays(window, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_dialog(&self, dialog: &Dialog, cx: &mut Context<Self>) -> AnyElement {
        let look = &self.look;
        let view = look.view();
        let parts = view.dialog();
        let item = self
            .snapshot
            .items
            .iter()
            .find(|item| item.key.canonical() == dialog.item);
        let host = match &self.request.target {
            herdr_client::ConnectTarget::Ssh { target, .. } => target.clone(),
            _ => "this machine".into(),
        };
        let profiles = std::iter::once((None, "built-in".to_owned())).chain(
            self.snapshot
                .profiles
                .iter()
                .enumerate()
                .map(|(index, profile)| (Some(index), profile.name.clone())),
        );
        // Many profiles wrap onto more rows of segments.
        let segmented = view
            .segments("orchestrator-profiles".into())
            .h_auto()
            .min_h(view.at(26.))
            .flex_wrap()
            .children(profiles.map(|(index, name)| {
                let selector = format!("orchestrator-profile-{name}");
                view.segment(
                    SharedString::from(selector.clone()),
                    selector,
                    name,
                    index == dialog.profile,
                )
                .h(view.at(22.))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(dialog) = &mut this.dialog {
                        dialog.profile = index;
                    }
                    cx.notify();
                }))
            }));
        let agents: Vec<AgentKind> = self
            .snapshot
            .installed
            .as_ref()
            .filter(|_| dialog.host == self.snapshot.installed_endpoint)
            .map(|installed| installed.iter().copied().take(8).collect())
            .unwrap_or_default();
        let tiles = if agents.is_empty() {
            let failed = dialog.host == self.snapshot.installed_endpoint
                && self.snapshot.installed_error.is_some();
            let text = if failed {
                "Could not discover agents. Select the host to retry."
            } else if self.snapshot.installed.is_none()
                || dialog.host != self.snapshot.installed_endpoint
            {
                "Looking for installed agents\u{2026}"
            } else {
                "No supported agent is installed on this host."
            };
            parts
                .note(
                    "orchestrator-agents-note".into(),
                    if failed {
                        NoteTone::Warning
                    } else {
                        NoteTone::Info
                    },
                    text,
                )
                .into_any_element()
        } else {
            let settings = view.settings();
            div()
                .flex()
                .flex_wrap()
                .gap(view.at(8.))
                .children(agents.into_iter().map(|kind| {
                    let on = dialog.kind == Some(kind);
                    let hover = view.hover();
                    let tile = div()
                        .id(SharedString::from(format!(
                            "orchestrator-agent-{}",
                            kind.name()
                        )))
                        .w(view.at(AGENT_TILE))
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(view.at(6.))
                        .py(view.at(10.))
                        .rounded(view.at(8.))
                        .border_1()
                        .border_color(view.line())
                        .bg(settings.card_fill())
                        .cursor_pointer()
                        .when(!on, |el| el.hover(move |style| style.bg(hover)))
                        .child(look.icon(agent_icon(kind.name()), 20., look.theme.foreground))
                        .child(
                            div()
                                .text_size(px(view.scale().secondary))
                                .text_color(view.title())
                                .child(kind.name()),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(dialog) = &mut this.dialog {
                                dialog.kind = Some(kind);
                            }
                            cx.notify();
                        }));
                    // The chosen agent wears the accent's ring.
                    if on { parts.ring(tile) } else { tile }
                }))
                .into_any_element()
        };
        let title = if dialog.review {
            "Review with agent"
        } else {
            "Dispatch agent"
        };
        let summary = item
            .map(|item| format!("{} {}", item.identifier, item.title))
            .unwrap_or_default();
        let close = div()
            .id("orchestrator-dialog-actions")
            .flex_none()
            .flex()
            .items_center()
            .gap(view.at(8.))
            .child(look.chip(format!("on {host}")))
            .child(
                look.icon_button("orchestrator-dialog-close", "icons/close.svg")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.close_overlays(window, cx);
                    })),
            );
        let mut rows: Vec<(&'static str, AnyElement)> =
            vec![("Prompt", segmented.into_any_element()), ("Agent", tiles)];
        if let Some(hosts) = self.render_hosts(dialog, cx) {
            rows.push(("Host", hosts));
        }
        rows.push(("Model", dialog.model.clone().into_any_element()));
        rows.push(("Branch", dialog.branch.clone().into_any_element()));
        div()
            .w(view.at(DIALOG_WIDTH))
            .flex()
            .flex_col()
            .child(parts.header(
                Header {
                    selector: "orchestrator-dialog-header",
                    icon: "icons/send.svg",
                    title: ("orchestrator-dialog-title", title),
                    sub: item.map(|item| {
                        (
                            if item.pull_request.is_some() {
                                "icons/git-branch.svg"
                            } else {
                                "icons/note.svg"
                            },
                            "orchestrator-dialog-item",
                            summary.as_str(),
                        )
                    }),
                    danger: false,
                },
                close,
            ))
            .child(
                parts
                    .body("orchestrator-dialog-body")
                    .child(parts.captions("orchestrator-dialog-captions", rows))
                    .child(
                        parts
                            .label("orchestrator-dialog-extra-label", "Extra instructions")
                            .child(dialog.extra.clone()),
                    )
                    .when_some(dialog.problem, |el, problem| {
                        el.child(parts.error("orchestrator-dialog-problem".into(), problem))
                    }),
            )
            .child(
                parts
                    .footer("orchestrator-dialog-footer")
                    .hint("Esc", "cancel")
                    .hint("\u{21b5}", "dispatch")
                    .button(
                        parts
                            .button(
                                "orchestrator-dialog-cancel",
                                "Cancel",
                                ButtonKind::Normal,
                                true,
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_overlays(window, cx);
                            })),
                    )
                    .button(
                        parts
                            .button(
                                "orchestrator-dialog-dispatch",
                                "Dispatch",
                                ButtonKind::Primary,
                                true,
                            )
                            .on_click(
                                cx.listener(|this, _, window, cx| this.submit_dispatch(window, cx)),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// Keys while the dialog is open: Enter dispatches, Escape cancels.
    pub(super) fn dialog_key(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match (key, self.dialog.is_some(), self.confirm.is_some()) {
            ("escape", true, _) | ("escape", _, true) => self.close_overlays(window, cx),
            ("enter", true, _) => {
                self.submit_dispatch(window, cx);
                true
            }
            _ => false,
        }
    }
}

/// The dialogs' widths at the orchestrator's face of 12, and an agent's
/// tile.
const DIALOG_WIDTH: f32 = 600.;
const CONFIRM_WIDTH: f32 = 420.;
const AGENT_TILE: f32 = 84.;
