//! The orchestrator's small drawing parts, in the window's theme and fonts:
//! badges, chips, buttons, status dots. Since the redesign's fifth phase
//! they are the views' shared pieces at the orchestrator's face, and runs
//! show their state in Herdr's colours.

use super::rows::Status;
use crate::{
    config::{FontConfig, Theme},
    design::{ButtonKind, Tone, ViewLook},
    fonts::StyledFont,
    sidebar::Indicators,
};
use gpui::{prelude::*, *};
use herdr_client::protocol::AgentStatus;

pub(crate) const RED: usize = 1;
pub(crate) const GREEN: usize = 2;
pub(crate) const YELLOW: usize = 3;
pub(crate) const BLUE: usize = 4;
pub(crate) const MAGENTA: usize = 5;
pub(crate) const CYAN: usize = 6;

/// What the view draws with; the window replaces it when settings change.
#[derive(Clone, Debug)]
pub(crate) struct Look {
    pub(crate) theme: Theme,
    pub(crate) ui: FontConfig,
    pub(crate) mono: FontConfig,
    /// Whether the theme is light, which softens the panels' shadows.
    pub(crate) light: bool,
    /// Herdr's status colours, the user's own included, for runs' states.
    pub(crate) indicators: Indicators,
    /// The redesigned views' pieces, built once with the look.
    view: ViewLook,
}

/// `label` with only its first letter capital, as the redesign's section
/// labels are: «LATEST RUN» reads «Latest run».
pub(crate) fn sentence(label: &str) -> String {
    let lower = label.to_lowercase();
    let mut letters = lower.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

impl Look {
    /// The look for `theme`, its text in `ui` and `mono`, and its chrome
    /// (bars, rows, panels) at `chrome`, the `[ui]` face every view's
    /// chrome shares.
    pub(crate) fn new(
        indicators: Indicators,
        theme: Theme,
        ui: FontConfig,
        mono: FontConfig,
        light: bool,
        chrome: &FontConfig,
    ) -> Self {
        let view = ViewLook::of(&theme, chrome, light);
        Self {
            theme,
            ui,
            mono,
            light,
            indicators,
            view,
        }
    }

    /// The redesigned views' pieces, at the chrome's face.
    pub(crate) fn view(&self) -> ViewLook {
        self.view
    }

    pub(crate) fn hue(&self, index: usize) -> u32 {
        self.theme.ink(self.theme.palette[index])
    }

    pub(crate) fn size(&self) -> Pixels {
        px(self.ui.size)
    }

    pub(crate) fn small(&self) -> Pixels {
        px(self.ui.size - 1.)
    }

    pub(crate) fn tiny(&self) -> Pixels {
        px((self.ui.size - 2.).max(9.))
    }

    /// A run's state in the sidebar's colours: working yellow, waiting red,
    /// idle green, done teal; a failure in the danger red, stopped grey.
    pub(crate) fn status_hue(&self, status: Status) -> u32 {
        match status {
            Status::Starting | Status::Working => self.indicators.color(AgentStatus::Working),
            Status::NeedsInput => self.indicators.color(AgentStatus::Blocked),
            Status::Idle => self.indicators.color(AgentStatus::Idle),
            Status::Done => self.indicators.color(AgentStatus::Done),
            Status::Failed => crate::menu::danger_value(&self.theme),
            Status::Stopped => self.theme.muted,
        }
    }

    /// A run's state as a chip: a failure in red, so it never reads as
    /// waiting, stopped in grey, the rest in their status colour.
    pub(crate) fn status_tone(&self, status: Status) -> Tone {
        match status {
            Status::Failed => Tone::Danger,
            Status::Stopped => Tone::Neutral,
            status => Tone::Status(self.status_hue(status)),
        }
    }

    pub(crate) fn icon(&self, path: &'static str, size: f32, color: u32) -> Svg {
        svg()
            .flex_none()
            .size(px(size))
            .path(path)
            .text_color(rgb(color))
    }

    /// A short coloured word, such as a state or priority, in its tone.
    pub(crate) fn badge(&self, text: impl Into<SharedString>, tone: Tone) -> Div {
        self.view()
            .settings()
            .chip("orchestrator-badge".to_owned(), text, tone)
    }

    /// A neutral tag, such as a label or host.
    pub(crate) fn chip(&self, text: impl Into<SharedString>) -> Div {
        self.badge(text, Tone::Neutral)
    }

    pub(crate) fn mono(&self, text: impl Into<SharedString>) -> Div {
        div()
            .text_font(&self.mono)
            .text_size(self.small())
            .child(text.into())
    }

    pub(crate) fn muted(&self, text: impl Into<SharedString>) -> Div {
        div()
            .text_color(rgb(self.theme.muted))
            .text_size(self.small())
            .child(text.into())
    }

    /// A section's label, quiet and in sentence case.
    pub(crate) fn label(&self, text: impl AsRef<str>) -> Div {
        let view = self.view();
        div()
            .text_size(px(view.scale().detail))
            .text_color(view.muted())
            .font_weight(FontWeight::MEDIUM)
            .child(sentence(text.as_ref()))
    }

    /// A small button; `primary` is the action a part leads with. Its
    /// selector is its id.
    pub(crate) fn button(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        primary: bool,
    ) -> Stateful<Div> {
        let id = id.into();
        let selector = id.to_string();
        self.view().button(
            id,
            selector,
            None,
            label,
            if primary {
                ButtonKind::Primary
            } else {
                ButtonKind::Normal
            },
            true,
        )
    }

    /// A square button holding one icon; its selector is its id.
    pub(crate) fn icon_button(
        &self,
        id: impl Into<ElementId>,
        path: &'static str,
    ) -> Stateful<Div> {
        let id = id.into();
        let selector = id.to_string();
        self.view().icon_button(id, selector, path, false, true)
    }

    /// A filter that turns on and off, as a pill; its selector is its id.
    pub(crate) fn toggle(
        &self,
        id: impl Into<ElementId>,
        label: &'static str,
        on: bool,
    ) -> Stateful<Div> {
        let id = id.into();
        let selector = id.to_string();
        self.view().pill(id, selector, label, on)
    }

    /// An item's state as a chip: open in green, under way in the accent,
    /// blocked in yellow, merged in mauve, closed in red.
    pub(crate) fn state_tone(&self, state: &str) -> Tone {
        match state {
            "open" => Tone::Good,
            "in_progress" | "in progress" | "draft" => Tone::Accent,
            "blocked" => Tone::Warn,
            "merged" => Tone::Merged,
            "closed" | "tombstone" => Tone::Danger,
            _ => Tone::Neutral,
        }
    }

    /// A Beads priority as a chip, P0 and P1 hottest.
    pub(crate) fn priority_tone(&self, priority: i64) -> Tone {
        match priority {
            0 | 1 => Tone::Danger,
            2 => Tone::Warn,
            _ => Tone::Neutral,
        }
    }
}

/// The icon of an agent by the harness name runs record.
pub(crate) fn agent_icon(agent: &str) -> &'static str {
    crate::icons::AgentIcon::from_identity(Some(agent)).path()
}

/// "2d", "5h", "12m": how long ago `at` was.
pub(crate) fn age(
    at: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let Some(at) = at else {
        return String::new();
    };
    let seconds = (now - at).num_seconds().max(0);
    match seconds {
        0..60 => "now".into(),
        60..3_600 => format!("{}m", seconds / 60),
        3_600..86_400 => format!("{}h", seconds / 3_600),
        86_400..1_209_600 => format!("{}d", seconds / 86_400),
        _ => format!("{}w", seconds / 604_800),
    }
}
