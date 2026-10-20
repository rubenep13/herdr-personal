//! Daemon-host integration management. No native credentials or local file writes.
use crate::{
    HerdrWindow,
    connection::IntegrationInbox,
    design::{ButtonKind, ButtonSize, SettingsLook, Tone},
    icons::AgentIcon,
};
use gpui::{prelude::*, *};
use herdr_client::Method;
use serde_json::Value;
use std::sync::{Arc, Mutex};

const MAX_ROWS: usize = 64;
const MAX_TEXT: usize = 1024;
const MAX_MESSAGES: usize = 32;

#[derive(Debug, thiserror::Error)]
enum ResponseError {
    #[error("Invalid integration response from the daemon.")]
    Invalid,
    #[error("Integration response exceeds the display limits.")]
    Limit,
    #[error("{code}: {message}")]
    Daemon { code: String, message: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    NotInstalled,
    Current,
    Outdated,
}

/// The colour a state's chip takes: well when current, a warning when an
/// update waits, quiet otherwise.
fn tone(state: State) -> Tone {
    match state {
        State::Current => Tone::Good,
        State::Outdated => Tone::Warn,
        State::NotInstalled => Tone::Neutral,
    }
}

#[derive(Debug)]
struct Integration {
    target: String,
    label: String,
    command: String,
    available: bool,
    state: State,
}

impl Integration {
    fn matches(&self, query: &str) -> bool {
        self.label.to_lowercase().contains(query) || self.target.to_lowercase().contains(query)
    }

    /// The agent's mark, found by its target, its command or the target's
    /// first word (`gemini_cli` is Gemini); unknown agents get the generic one.
    fn icon(&self) -> AgentIcon {
        [
            self.target.as_str(),
            self.command.as_str(),
            self.target.split('_').next().unwrap_or_default(),
        ]
        .into_iter()
        .map(|identity| AgentIcon::from_identity(Some(identity)))
        .find(|icon| *icon != AgentIcon::Generic)
        .unwrap_or(AgentIcon::Generic)
    }
}

fn text(value: &Value, limit: usize) -> Result<&str, ResponseError> {
    let text = value.as_str().ok_or(ResponseError::Invalid)?;
    if text.len() > limit {
        return Err(ResponseError::Limit);
    }
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(ResponseError::Invalid);
    }
    Ok(text)
}

fn result<'a>(response: &'a Value, kind: &str) -> Result<&'a Value, ResponseError> {
    if let Some(error) = response.get("error").filter(|error| !error.is_null()) {
        return Err(ResponseError::Daemon {
            code: text(&error["code"], 128)?.into(),
            message: text(&error["message"], MAX_TEXT)?.into(),
        });
    }
    let result = &response["result"];
    if result["type"] != kind {
        return Err(ResponseError::Invalid);
    }
    Ok(result)
}

fn parse_list(response: &Value) -> Result<Vec<Integration>, ResponseError> {
    let rows = result(response, "integration_list")?["integrations"]
        .as_array()
        .ok_or(ResponseError::Invalid)?;
    if rows.len() > MAX_ROWS {
        return Err(ResponseError::Limit);
    }
    let mut integrations = Vec::<Integration>::with_capacity(rows.len());
    for row in rows {
        let target = text(&row["target"], 64)?;
        if !target.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
            || integrations.iter().any(|item| item.target == target)
        {
            return Err(ResponseError::Invalid);
        }
        integrations.push(Integration {
            target: target.into(),
            label: text(&row["label"], 256)?.into(),
            command: text(&row["command"], MAX_TEXT)?.into(),
            available: row["available"].as_bool().ok_or(ResponseError::Invalid)?,
            state: match row["state"].as_str() {
                Some("not_installed") => State::NotInstalled,
                Some("current") => State::Current,
                Some("outdated") => State::Outdated,
                _ => return Err(ResponseError::Invalid),
            },
        });
    }
    integrations.sort_by_cached_key(|row| (row.label.to_lowercase(), row.target.clone()));
    Ok(integrations)
}

fn parse_install(response: &Value, target: &str) -> Result<Vec<String>, ResponseError> {
    let result = result(response, "integration_install")?;
    if text(&result["target"], 64)? != target {
        return Err(ResponseError::Invalid);
    }
    let messages = result["details"]["messages"]
        .as_array()
        .ok_or(ResponseError::Invalid)?;
    if messages.len() > MAX_MESSAGES {
        return Err(ResponseError::Limit);
    }
    messages
        .iter()
        .map(|value| text(value, MAX_TEXT).map(str::to_owned))
        .collect()
}

#[derive(Clone)]
struct Scope {
    endpoint: String,
    host: String,
    epoch: u64,
    generation: u64,
    boot: String,
    inbox: Arc<Mutex<IntegrationInbox>>,
}

#[derive(Default)]
pub(crate) struct Integrations {
    pub busy: bool,
    pub error: Option<String>,
    pub messages: Vec<String>,
    rows: Vec<Integration>,
    scope: Option<Scope>,
    pending: Option<(String, Operation)>,
    scroll: ScrollHandle,
}

#[derive(Clone)]
enum Operation {
    List,
    Install(String),
}

impl Integrations {
    /// The listed integrations as `(target, label, state)`; the state is
    /// "not_installed", "current", "outdated", or "unavailable".
    #[cfg(unix)]
    pub(crate) fn listed(&self) -> impl Iterator<Item = (&str, &str, &'static str)> {
        self.rows.iter().map(|row| {
            let state = match row.state {
                _ if !row.available => "unavailable",
                State::NotInstalled => "not_installed",
                State::Current => "current",
                State::Outdated => "outdated",
            };
            (row.target.as_str(), row.label.as_str(), state)
        })
    }

    fn clear_pending(&mut self) {
        if let Some((id, _)) = self.pending.take()
            && let Some(scope) = &self.scope
            && let Ok(mut inbox) = scope.inbox.lock()
            && inbox
                .pending
                .as_ref()
                .is_some_and(|(request, _)| *request == id)
        {
            inbox.pending = None;
        }
        self.busy = false;
    }
}

impl HerdrWindow {
    fn integration_scope_current(&self, scope: &Scope) -> bool {
        self.endpoints
            .get(self.selected_endpoint)
            .is_some_and(|endpoint| {
                endpoint.id == scope.endpoint
                    && endpoint.generation == scope.generation
                    && self.selection_epoch == scope.epoch
                    && Arc::ptr_eq(&endpoint.connection.integrations, &scope.inbox)
                    && endpoint.live.status.is_connected()
                    && endpoint
                        .live
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.boot_id == scope.boot)
            })
    }

    pub(crate) fn load_integrations(&mut self, cx: &mut Context<Self>) {
        if self.integrations.busy
            && self
                .integrations
                .scope
                .as_ref()
                .is_some_and(|scope| self.integration_scope_current(scope))
        {
            return;
        }
        self.integrations.clear_pending();
        self.integrations.rows.clear();
        self.integrations.messages.clear();
        self.integrations.error = None;
        self.integrations.scope = None;
        let request = (|| {
            let endpoint = self
                .endpoints
                .get(self.selected_endpoint)
                .ok_or(crate::Error::NotConnected)?;
            if !endpoint.live.status.is_connected() {
                return Err(crate::Error::NotConnected);
            }
            let snapshot = endpoint
                .live
                .snapshot
                .as_ref()
                .ok_or(crate::Error::NoSnapshot)?;
            let scope = Scope {
                endpoint: endpoint.id.clone(),
                host: endpoint.label.clone(),
                epoch: self.selection_epoch,
                generation: endpoint.generation,
                boot: snapshot.boot_id.clone(),
                inbox: endpoint.connection.integrations.clone(),
            };
            let id = endpoint.connection.request_integration(
                &scope.boot,
                Method::IntegrationList,
                serde_json::json!({}),
            )?;
            Ok::<_, crate::Error>((scope, id))
        })();
        match request {
            Ok((scope, id)) => {
                self.integrations.scope = Some(scope);
                self.integrations.pending = Some((id, Operation::List));
                self.integrations.busy = true;
            }
            Err(error) => self.integrations.error = Some(error.to_string()),
        }
        cx.notify();
    }

    /// Only call from an explicit install/reinstall click, never panel opening.
    pub(crate) fn install_integration(&mut self, target: String, cx: &mut Context<Self>) {
        if self.integrations.busy {
            return;
        }
        let Some(scope) = &self.integrations.scope else {
            return;
        };
        if !self.integration_scope_current(scope)
            || !self
                .integrations
                .rows
                .iter()
                .any(|row| row.target == target && row.available)
        {
            self.integrations.error =
                Some("The host or integration list changed. Refresh before installing.".into());
            cx.notify();
            return;
        }
        self.integrations.error = None;
        self.integrations.messages.clear();
        match self.endpoints[self.selected_endpoint]
            .connection
            .request_integration(
                &scope.boot,
                Method::IntegrationInstall,
                serde_json::json!({"target": target}),
            ) {
            Ok(id) => {
                self.integrations.pending = Some((id, Operation::Install(target)));
                self.integrations.busy = true;
            }
            Err(error) => self.integrations.error = Some(error.to_string()),
        }
        cx.notify();
    }

    /// Run after endpoint polling, including while the panel is closed.
    pub(crate) fn poll_integrations(&mut self, cx: &mut Context<Self>) {
        let Some(scope) = &self.integrations.scope else {
            return;
        };
        if !self.integration_scope_current(scope) {
            self.integrations.clear_pending();
            self.integrations.scope = None;
            self.integrations.rows.clear();
            self.integrations.messages.clear();
            self.integrations.error = Some("The selected daemon changed or disconnected. Refresh to load integrations. An already queued install may have completed on the previous host.".into());
            cx.notify();
            return;
        }
        let Some((id, operation)) = &self.integrations.pending else {
            return;
        };
        let response = scope.inbox.try_lock().ok().and_then(|mut inbox| {
            let (request, response) = inbox.pending.as_mut()?;
            (request == id).then(|| response.take()).flatten()
        });
        let Some(response) = response else { return };
        let operation = operation.clone();
        self.integrations.clear_pending();
        match response {
            Err(error) => self.integrations.error = Some(error.to_string()),
            Ok(response) => match operation {
                Operation::List => match parse_list(&response) {
                    Ok(rows) => self.integrations.rows = rows,
                    Err(error) => self.integrations.error = Some(error.to_string()),
                },
                Operation::Install(target) => match parse_install(&response, &target) {
                    Ok(messages) => {
                        self.load_integrations(cx);
                        self.integrations.messages = messages;
                    }
                    Err(error) => self.integrations.error = Some(error.to_string()),
                },
            },
        }
        cx.notify();
    }

    pub(crate) fn render_filtered_integrations(&self, query: &str, cx: &mut Context<Self>) -> Div {
        let query = query.trim().to_lowercase();
        let state = &self.integrations;
        let theme = &self.theme;
        let look = SettingsLook::of(theme, &self.config.ui);
        let scale = look.scale();
        let current = state
            .scope
            .as_ref()
            .filter(|scope| self.integration_scope_current(scope));
        let host = current
            .map(|scope| scope.host.as_str())
            .or_else(|| {
                self.endpoints
                    .get(self.selected_endpoint)
                    .map(|endpoint| endpoint.label.as_str())
            })
            .unwrap_or("No selected host");
        let can_install = !state.busy
            && current.is_some_and(|scope| scope.inbox.try_lock().is_ok_and(|inbox| inbox.install));
        let epoch = self.selection_epoch;
        let note = |text: SharedString| {
            div()
                .mt(look.at(8.))
                .text_size(px(scale.secondary))
                .text_color(look.muted())
                .child(text)
        };
        // The daemon's host is the section's title, and what Install does
        // there its lead.
        let mut body = div()
            .id("integrations-body")
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&state.scroll)
            .child(
                div()
                    .mt(look.at(24.))
                    .mx(look.at(2.))
                    .text_size(px(scale.secondary))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(look.text())
                    .child(crate::sidebar::label_text(&format!("Daemon host: {host}"))),
            )
            .child(
                div()
                    .mt(look.at(4.))
                    .mx(look.at(2.))
                    .text_size(px(scale.secondary))
                    .line_height(relative(1.5))
                    .text_color(look.muted())
                    .child(
                        "Install updates agent configuration on this daemon's host, not \
                         necessarily this computer. No integrations are installed automatically.",
                    ),
            );
        if let Some(error) = &state.error {
            body = body.child(note(error.clone().into()).text_color(look.danger()));
        }
        if state.busy {
            body = body.child(note("Working on the selected daemon...".into()));
        } else {
            body = body.child(
                div().mt(look.at(8.)).flex().child(
                    look.button(
                        "integrations-refresh",
                        "integrations-refresh".into(),
                        "Refresh integrations",
                        ButtonKind::Normal,
                        ButtonSize::Small,
                        true,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.selection_epoch == epoch {
                            this.load_integrations(cx);
                        }
                    })),
                ),
            );
        }
        if let Some(scope) = current {
            if !state.busy && state.rows.is_empty() && state.error.is_none() {
                body = body.child(note("No integrations reported by this daemon.".into()));
            }
            let rows: Vec<_> = state
                .rows
                .iter()
                .filter(|row| row.matches(&query))
                .collect();
            if rows.is_empty() && !state.rows.is_empty() {
                body = body.child(note("No matching integrations.".into()));
            }
            if !rows.is_empty() {
                let mut card = look.card("integrations-card".into()).mt(look.at(10.));
                for (index, row) in rows.into_iter().enumerate() {
                    if index > 0 {
                        card = card.child(look.rule());
                    }
                    card = card.child(self.integration_row(row, &look, can_install, scope, cx));
                }
                body = body.child(card);
            }
            for message in &state.messages {
                body = body.child(note(message.clone().into()));
            }
        }
        div().flex().flex_col().min_h_0().child(body)
    }

    /// One integration: its agent's tile, its name over its command, its
    /// state in a chip and its action.
    fn integration_row(
        &self,
        row: &Integration,
        look: &SettingsLook,
        can_install: bool,
        scope: &Scope,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = &self.theme;
        let scale = look.scale();
        let target = row.target.clone();
        let status = match row.state {
            State::NotInstalled => "Not installed",
            State::Current => "Current",
            State::Outdated => "Outdated",
        };
        let icon = row.icon();
        let line = |text: &'static str| {
            div()
                .text_size(px(scale.secondary))
                .text_color(look.muted())
                .child(text)
        };
        let reason = if !row.available {
            Some("Agent not available on this host")
        } else if !can_install && !self.integrations.busy {
            Some("Installation not supported by this daemon")
        } else {
            None
        };
        div()
            .debug_selector({
                let target = target.clone();
                move || format!("integration-{target}")
            })
            .flex()
            .items_center()
            .gap(look.at(12.))
            .min_h(px(look.row_min_height()))
            .px(look.at(14.))
            .py(look.at(9.))
            .child(crate::sidebar::tile(
                format!("integration-{target}-tile"),
                icon.path(),
                crate::sidebar::logo_color(icon, theme),
                false,
                theme,
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .truncate()
                            .text_size(px(scale.name))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(look.title())
                            .child(row.label.clone()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(scale.secondary))
                            .text_color(look.muted())
                            .child(row.command.clone()),
                    )
                    .children(reason.map(line)),
            )
            .child(look.chip(
                format!("integration-{target}-state"),
                status,
                tone(row.state),
            ))
            .when(row.available && can_install, |item| {
                let scope = scope.clone();
                let action = target.clone();
                item.child(
                    look.button(
                        SharedString::from(format!("install-{target}")),
                        format!("install-{target}"),
                        if row.state == State::NotInstalled {
                            "Install"
                        } else {
                            "Reinstall"
                        },
                        ButtonKind::Normal,
                        ButtonSize::Small,
                        true,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.integration_scope_current(&scope) {
                            this.install_integration(action.clone(), cx);
                        }
                    })),
                )
            })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    use serde_json::json;

    fn list() -> Value {
        json!({"result":{"type":"integration_list","integrations":[{
            "target":"antigravity_cli","label":"Antigravity CLI","command":"antigravity",
            "available":true,"state":"outdated"
        }]}})
    }

    #[test]
    fn list_sort_and_search_keep_original_action_targets() {
        let response = json!({"result":{"type":"integration_list","integrations":[
            {"target":"zebra","label":"alpha","command":"z","available":true,"state":"current"},
            {"target":"beta","label":"Beta","command":"b","available":true,"state":"current"},
            {"target":"alpha","label":"ALPHA","command":"a","available":true,"state":"current"}
        ]}});
        let rows = parse_list(&response).unwrap();
        assert_eq!(
            rows.iter()
                .map(|row| row.target.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zebra", "beta"]
        );
        assert_eq!(
            rows.iter()
                .filter(|row| row.matches("alpha"))
                .map(|row| row.target.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zebra"]
        );
        assert!(rows[1].matches("zebra"));
        assert!(!rows[1].matches("missing"));
        assert!(rows.iter().all(|row| row.matches("")));
    }

    #[test]
    fn list_parses_upstream_states_and_targets() {
        let mut response = list();
        for (wire, state) in [
            ("current", State::Current),
            ("outdated", State::Outdated),
            ("not_installed", State::NotInstalled),
        ] {
            response["result"]["integrations"][0]["state"] = json!(wire);
            let rows = parse_list(&response).unwrap();
            assert_eq!(rows[0].state, state);
            assert_eq!(rows[0].target, "antigravity_cli");
        }
    }

    #[test]
    fn rejects_malformed_duplicate_and_oversized_lists_atomically() {
        for (field, value) in [
            ("state", json!("future")),
            ("available", json!("true")),
            ("label", json!("x".repeat(257))),
            ("command", json!("bad\u{1b}command")),
            ("target", json!("../claude")),
        ] {
            let mut response = list();
            response["result"]["integrations"][0][field] = value;
            assert!(parse_list(&response).is_err());
        }
        let mut response = list();
        let row = response["result"]["integrations"][0].clone();
        response["result"]["integrations"] = json!([row.clone(), row.clone()]);
        assert!(matches!(parse_list(&response), Err(ResponseError::Invalid)));
        response["result"]["integrations"] = json!(vec![row; MAX_ROWS + 1]);
        assert!(matches!(parse_list(&response), Err(ResponseError::Limit)));
        assert!(parse_list(&json!({"result": {"integrations": []}})).is_err());
    }

    #[test]
    fn install_requires_matching_target_and_bounded_messages() {
        let mut response = json!({"result":{"type":"integration_install","target":"claude","details":{"messages":["Installed"]}}});
        assert_eq!(parse_install(&response, "claude").unwrap(), ["Installed"]);
        assert!(parse_install(&response, "codex").is_err());
        response["result"]["details"]["messages"] = json!(vec!["x"; MAX_MESSAGES + 1]);
        assert!(matches!(
            parse_install(&response, "claude"),
            Err(ResponseError::Limit)
        ));
        response["result"]["details"]["messages"] = json!(["x".repeat(MAX_TEXT + 1)]);
        assert!(matches!(
            parse_install(&response, "claude"),
            Err(ResponseError::Limit)
        ));
    }

    #[test]
    fn daemon_errors_are_validated_before_display() {
        let response =
            json!({"error":{"code":"integration_install_failed","message":"Permission denied"}});
        assert!(matches!(
            parse_list(&response),
            Err(ResponseError::Daemon { .. })
        ));
        let response = json!({"error":{"code":"failure","message":"x".repeat(MAX_TEXT + 1)}});
        assert!(matches!(parse_list(&response), Err(ResponseError::Limit)));
    }

    mod look;
}
