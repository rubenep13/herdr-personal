//! The workspace menu's "Start agent...": choosing one of the agents the
//! space's host has installed, starting it on a worker, and applying what
//! came back only to the exact endpoint, session, boot, and connection it
//! was started for.

use super::{DEADLINE, StartAgent, StartOutcome, agent_name, start_agent};
use crate::{
    HerdrWindow, NavigationTarget,
    menu::Page,
    sidebar::tree::EndpointKey,
    teleport::{AgentKind, Host, host_for},
    window::Flash,
};
use gpui::{prelude::*, *};
use herdr_client::ConnectTarget;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// The space a start is for, as it was when the user asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Target {
    endpoint_id: String,
    key: EndpointKey,
    connect: ConnectTarget,
    generation: u64,
    boot_id: String,
    workspace_id: String,
    label: String,
}

/// The agents found on the host, while the menu is choosing one.
#[derive(Debug)]
enum Agents {
    Looking,
    Found(Vec<AgentKind>),
    Failed(String),
}

#[derive(Debug)]
struct Choosing {
    target: Target,
    agents: Agents,
}

/// Stops the worker's script when the start is dropped, as when the window
/// closes. The worker runs on; nothing waits for it.
#[derive(Debug, Default)]
struct Cancel(Arc<AtomicBool>);

impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[derive(Debug)]
struct Running {
    kind: AgentKind,
    _cancel: Cancel,
}

/// The window's agent start: the menu choosing one, and the one start in
/// flight, if any.
#[derive(Debug, Default)]
pub(crate) struct AgentStart {
    choosing: Option<Choosing>,
    running: Option<Running>,
}

impl HerdrWindow {
    /// Whether the workspace menu offers to start an agent: its host can be
    /// scripted from this client and no other start is in flight.
    pub(crate) fn start_agent_available(&self) -> bool {
        cfg!(any(target_os = "linux", target_os = "macos"))
            && self.agent_start.running.is_none()
            && self.menu_space().is_some()
            && host_for(&self.endpoints[self.selected_endpoint].connection.target).is_ok()
    }

    /// The menu's space, as a start's target.
    fn start_target(&self) -> Option<Target> {
        let (boot_id, workspace_id, label) = self.menu_space()?;
        let endpoint = &self.endpoints[self.selected_endpoint];
        Some(Target {
            endpoint_id: endpoint.id.clone(),
            key: EndpointKey::new(&endpoint.id, &endpoint.connection.target),
            connect: endpoint.connection.target.clone(),
            generation: endpoint.generation,
            boot_id: boot_id.to_owned(),
            workspace_id: workspace_id.to_owned(),
            label: label.to_owned(),
        })
    }

    /// Opens the agent choice for the menu's space and looks, off the UI
    /// thread, for the agents its host has installed.
    pub(crate) fn open_start_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.start_target() else {
            return;
        };
        let Ok(host) = host_for(&target.connect) else {
            return;
        };
        self.agent_start.choosing = Some(Choosing {
            target: target.clone(),
            agents: Agents::Looking,
        });
        self.menu.page = Some(Page::StartAgent);
        self.menu.error = None;
        window.focus(&self.menu.focus, cx);
        cx.notify();
        let cancel = Arc::new(AtomicBool::new(false));
        let looking = cx
            .background_executor()
            .spawn(async move { crate::fan_out::installed(&host, &cancel) });
        cx.spawn(async move |this, cx| {
            let found = looking.await;
            let _ = this.update(cx, |this, cx| {
                let Some(choosing) = this
                    .agent_start
                    .choosing
                    .as_mut()
                    .filter(|choosing| choosing.target == target)
                else {
                    return;
                };
                choosing.agents = match found {
                    Ok(kinds) => Agents::Found(kinds),
                    Err(error) => Agents::Failed(error.to_string()),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Starts `kind` in the chosen space. The menu closes; the outcome
    /// arrives as a flash.
    pub(crate) fn start_agent_kind(
        &mut self,
        kind: AgentKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.agent_start.running.is_some() {
            return;
        }
        let Some(target) = self
            .agent_start
            .choosing
            .take()
            .map(|choosing| choosing.target)
        else {
            return;
        };
        let Ok(host) = host_for(&target.connect) else {
            return;
        };
        self.dismiss_menu(window, cx);
        let cancel = Cancel::default();
        let flag = cancel.0.clone();
        self.agent_start.running = Some(Running {
            kind,
            _cancel: cancel,
        });
        self.show_flash(
            Flash::success(format!("Starting {} in {}…", kind.name(), target.label)),
            cx,
        );
        let request = StartAgent {
            workspace_id: target.workspace_id.clone(),
            kind,
            name: agent_name(kind),
        };
        let started = cx
            .background_executor()
            .spawn(async move { run(&host, &request, &flag) });
        cx.spawn(async move |this, cx| {
            let outcome = started.await;
            let _ = this.update(cx, |this, cx| this.finish_agent_start(&target, outcome, cx));
        })
        .detach();
    }

    /// Whether `target`'s endpoint is still the one the start was made for:
    /// the same session, boot, and connection, so its pane IDs still name
    /// the same panes.
    fn start_still_applies(&self, target: &Target) -> bool {
        let Some(index) = self
            .endpoints
            .iter()
            .position(|endpoint| endpoint.id == target.endpoint_id)
        else {
            return false;
        };
        let endpoint = &self.endpoints[index];
        let live = if index == self.selected_endpoint {
            &self.live
        } else {
            &endpoint.live
        };
        endpoint.generation == target.generation
            && EndpointKey::new(&endpoint.id, &endpoint.connection.target) == target.key
            && live
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.boot_id == target.boot_id)
    }

    /// Reports a finished start, focusing its pane only while the endpoint
    /// it was made for still holds it. Never starts anything again.
    pub(crate) fn finish_agent_start(
        &mut self,
        target: &Target,
        outcome: StartOutcome,
        cx: &mut Context<Self>,
    ) {
        let kind = self
            .agent_start
            .running
            .take()
            .map_or("The agent", |running| running.kind.name());
        let applies = self.start_still_applies(target);
        let flash = match outcome {
            StartOutcome::Ready { pane_id } if applies => {
                self.navigate_endpoint(&target.endpoint_id, NavigationTarget::Pane(&pane_id), cx);
                Flash::success(format!("{kind} is ready in {}.", target.label))
            }
            StartOutcome::Ready { .. } => Flash::warning(format!(
                "{kind} started, but {}'s host changed since; its pane was not focused.",
                target.label
            )),
            StartOutcome::Failed(error) => {
                Flash::warning(format!("{kind} was not started: {error}"))
            }
            StartOutcome::Unknown { pane_id, error } => {
                // The tab is this boot's own, so it can show what happened.
                if applies {
                    self.navigate_endpoint(
                        &target.endpoint_id,
                        NavigationTarget::Pane(&pane_id),
                        cx,
                    );
                }
                Flash::warning(format!(
                    "Could not confirm {kind} started in {} ({error}). It was not retried; the space shows what runs.",
                    target.label
                ))
            }
        };
        self.show_flash(flash, cx);
        cx.notify();
    }

    /// The agent choice: the host's installed agents, one button each.
    pub(crate) fn render_start_agent(&self, cx: &mut Context<Self>) -> Div {
        let theme = &self.theme;
        let Some(choosing) = &self.agent_start.choosing else {
            return div();
        };
        let body = match &choosing.agents {
            Agents::Looking => div()
                .p(px(8.))
                .text_color(rgb(theme.muted))
                .child("Looking for agents on this host…"),
            Agents::Failed(error) => div()
                .p(px(8.))
                .text_color(rgb(theme.muted))
                .child(format!("Could not list the agents: {error}")),
            Agents::Found(kinds) if kinds.is_empty() => div()
                .p(px(8.))
                .text_color(rgb(theme.muted))
                .child("No agent Herdr can start is installed on this host."),
            Agents::Found(kinds) => {
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.))
                    .p(px(8.))
                    .children(kinds.iter().map(|&kind| {
                        let id = SharedString::from(format!("start-agent-{}", kind.name()));
                        div()
                            .id(id.clone())
                            .debug_selector(move || id.to_string())
                            .p(px(8.))
                            .rounded(px(crate::config::corners::CONTROL))
                            .hover(|button| button.bg(rgb(theme.active)))
                            .cursor_pointer()
                            .child(kind.name())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.start_agent_kind(kind, window, cx);
                            }))
                    }))
            }
        };
        div()
            .debug_selector(|| "start-agent".into())
            .child(
                div()
                    .p(px(8.))
                    .child(format!("Start an agent in {}", choosing.target.label)),
            )
            .child(div().p(px(8.)).text_color(rgb(theme.muted)).child(
                "It opens in a new tab of this space, through Herdr's own agent start. Closing the app leaves it running.",
            ))
            .child(body)
    }
}

/// The blocking start, for the background executor.
fn run(host: &Host, request: &StartAgent, cancelled: &AtomicBool) -> StartOutcome {
    start_agent(host, request, cancelled, DEADLINE)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
