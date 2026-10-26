//! Runs show their state in Herdr's colours, the user's own included.
use super::*;
use crate::{
    config::{Config, Theme},
    design::Tone,
    orchestrator::view::Look,
    sidebar::Indicators,
};

fn look(settings: Option<&crate::herdr_settings::Settings>) -> Look {
    let config = Config::default();
    let theme = Theme::default();
    Look::new(
        Indicators::new(settings, None, false, &theme),
        theme,
        config.ui.clone(),
        config.terminal.clone(),
        false,
        &config.ui,
    )
}

#[test]
fn runs_take_herdrs_status_colours() {
    let look = look(None);
    let herdr = |status| look.indicators.color(status);
    assert_eq!(
        look.status_hue(Status::Working),
        herdr(AgentStatus::Working)
    );
    assert_eq!(
        look.status_hue(Status::Starting),
        herdr(AgentStatus::Working)
    );
    assert_eq!(
        look.status_hue(Status::NeedsInput),
        herdr(AgentStatus::Blocked)
    );
    assert_eq!(look.status_hue(Status::Idle), herdr(AgentStatus::Idle));
    assert_eq!(look.status_hue(Status::Done), herdr(AgentStatus::Done));
    assert_eq!(look.status_hue(Status::Stopped), look.theme.muted);
}

#[test]
fn runs_follow_the_users_colours() {
    let settings =
        crate::herdr_settings::Settings::parse_text("[theme.custom]\nred='#ff00aa'\n").unwrap();
    let look = look(Some(&settings));
    assert_eq!(
        look.status_hue(Status::NeedsInput),
        look.theme.ink(0xff00aa)
    );
}

#[test]
fn a_failed_run_says_so_in_red() {
    let look = look(None);
    assert_eq!(
        look.status_hue(Status::Failed),
        crate::menu::danger_value(&look.theme)
    );
    assert_eq!(look.status_tone(Status::Failed), Tone::Danger);
    assert_eq!(look.status_tone(Status::Stopped), Tone::Neutral);
    assert_eq!(
        look.status_tone(Status::Working),
        Tone::Status(look.status_hue(Status::Working))
    );
}

#[test]
fn states_and_priorities_take_their_tones() {
    let look = look(None);
    for (state, tone) in [
        ("open", Tone::Good),
        ("in_progress", Tone::Accent),
        ("draft", Tone::Accent),
        ("blocked", Tone::Warn),
        ("merged", Tone::Merged),
        ("closed", Tone::Danger),
        ("deferred", Tone::Neutral),
    ] {
        assert_eq!(look.state_tone(state), tone, "{state}");
    }
    for (priority, tone) in [
        (0, Tone::Danger),
        (1, Tone::Danger),
        (2, Tone::Warn),
        (3, Tone::Neutral),
    ] {
        assert_eq!(look.priority_tone(priority), tone, "P{priority}");
    }
}

/// Herdr's settings load after the tab is built: their colours reach the
/// view even when the theme stays the same.
#[gpui::test]
fn colours_that_arrive_later_reach_the_runs(cx: &mut gpui::TestAppContext) {
    use crate::orchestrator::{Request, Timing, view::OrchestratorView};
    let dir = tempfile::tempdir().unwrap();
    let request = Request {
        target: herdr_client::ConnectTarget::Local,
        checkout: dir.path().to_string_lossy().into_owned(),
        workspace_id: "w1".into(),
        token: None,
        data_root: Some(dir.path().join("data")),
        timing: Timing::default(),
    };
    let (view, cx) = cx.add_window_view(|_, cx| OrchestratorView::new(request, look(None), cx));
    let settings =
        crate::herdr_settings::Settings::parse_text("[theme.custom]\nred='#ff00aa'\n").unwrap();
    let mine = look(Some(&settings));
    view.update(cx, |view, cx| view.set_look(mine.clone(), cx));
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.look.status_hue(Status::NeedsInput),
            mine.status_hue(Status::NeedsInput)
        );
    });
}

/// The inbox's bar is as tall as every other view's at the default sizes,
/// and grows as the terminal's font grows.
#[gpui::test]
fn the_inbox_chrome_matches_the_other_views(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update(cx, |view, _| {
        assert_eq!(view.look().view().at(40.), gpui::px(40.));
        view.config.terminal.size = view.configured_terminal_size + 7.;
        assert!(view.look().view().at(40.) > gpui::px(40.));
    });
}
