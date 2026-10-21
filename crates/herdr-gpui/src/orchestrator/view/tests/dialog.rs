//! The dispatch dialog fits short windows and large text by scrolling.
use super::*;
use crate::orchestrator::{
    Request, Timing,
    view::{Look, OrchestratorView},
};
use gpui::{TestAppContext, px, size};
use std::sync::Arc;

#[gpui::test]
fn the_dispatch_dialog_stays_within_a_short_window(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let request = Request {
        target: herdr_client::ConnectTarget::Local,
        checkout: dir.path().to_string_lossy().into_owned(),
        workspace_id: "w1".into(),
        token: None,
        data_root: Some(dir.path().join("data")),
        timing: Timing::default(),
    };
    let config = crate::config::Config::default();
    let mut ui = config.ui.clone();
    ui.size = 48.;
    let look = Look::new(
        crate::sidebar::Indicators::new(None, None, false, &crate::config::Theme::default()),
        crate::config::Theme::default(),
        ui.clone(),
        config.terminal.clone(),
        false,
        &ui,
    );
    let (view, cx) = cx.add_window_view(|_, cx| OrchestratorView::new(request, look, cx));
    cx.simulate_resize(size(px(640.), px(400.)));
    let issue = item(Provider::Github, "377", "Icons", "2026-10-04T00:00:00Z");
    let key = issue.key.canonical();
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.snapshot.items = Arc::new(vec![issue]);
            view.refresh_rows();
            view.open_dispatch(key, window, cx);
        })
    });
    cx.run_until_parked();
    let card = cx
        .debug_bounds("orchestrator-overlay-card")
        .unwrap_or_else(|| panic!("the dialog's card"));
    assert!(
        card.top() >= px(0.) && card.bottom() <= px(400.),
        "{card:?}"
    );
}

#[gpui::test]
fn the_dispatch_dialog_has_a_header_and_footer(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let request = Request {
        target: herdr_client::ConnectTarget::Local,
        checkout: dir.path().to_string_lossy().into_owned(),
        workspace_id: "w1".into(),
        token: None,
        data_root: Some(dir.path().join("data")),
        timing: Timing::default(),
    };
    let config = crate::config::Config::default();
    let theme = crate::config::Theme::default();
    let look = Look::new(
        crate::sidebar::Indicators::new(None, None, false, &theme),
        theme,
        config.ui.clone(),
        config.terminal.clone(),
        false,
        &config.ui,
    );
    let (view, cx) = cx.add_window_view(|_, cx| OrchestratorView::new(request, look, cx));
    cx.simulate_resize(size(px(1200.), px(900.)));
    let issue = item(Provider::Github, "377", "Icons", "2026-10-04T00:00:00Z");
    let key = issue.key.canonical();
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.snapshot.items = Arc::new(vec![issue]);
            view.refresh_rows();
            view.open_dispatch(key, window, cx);
        })
    });
    cx.run_until_parked();
    let card = cx.debug_bounds("orchestrator-overlay-card").unwrap();
    let close = cx.debug_bounds("orchestrator-dialog-close").unwrap();
    assert!(
        card.right() - close.right() <= px(12.),
        "{close:?} in {card:?}"
    );
    assert!(cx.debug_bounds("orchestrator-dialog-header").is_some());
    let cancel = cx.debug_bounds("orchestrator-dialog-cancel").unwrap();
    let dispatch = cx.debug_bounds("orchestrator-dialog-dispatch").unwrap();
    assert!(dispatch.left() > cancel.left());
    assert_eq!(cancel.size.height, px(30.));
    assert_eq!(dispatch.size.height, px(30.));
    let footer = cx.debug_bounds("orchestrator-dialog-footer").unwrap();
    assert!(footer.contains(&dispatch.center()));
}

#[test]
fn a_confirmation_destroys_in_red() {
    use crate::{
        design::ButtonKind,
        orchestrator::view::dispatch::{Confirm, verb_kind},
    };
    let stop = Confirm::Stop {
        run: "r1".into(),
        agent: "claude".into(),
    };
    assert_eq!(verb_kind(&stop), ButtonKind::Danger);
    let remove = Confirm::Remove {
        run: "r1".into(),
        branch: "agent/1".into(),
    };
    assert_eq!(verb_kind(&remove), ButtonKind::Danger);
}

/// At large type in a narrow window the dialog narrows to the view, so its
/// cross and its buttons stay in reach.
#[gpui::test]
fn the_dispatch_dialog_narrows_to_the_window(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let request = Request {
        target: herdr_client::ConnectTarget::Local,
        checkout: dir.path().to_string_lossy().into_owned(),
        workspace_id: "w1".into(),
        token: None,
        data_root: Some(dir.path().join("data")),
        timing: Timing::default(),
    };
    let config = crate::config::Config::default();
    let theme = crate::config::Theme::default();
    let mut ui = config.ui.clone();
    ui.size = 18.;
    let look = Look::new(
        crate::sidebar::Indicators::new(None, None, false, &theme),
        theme,
        ui.clone(),
        config.terminal.clone(),
        false,
        &ui,
    );
    let (view, cx) = cx.add_window_view(|_, cx| OrchestratorView::new(request, look, cx));
    cx.simulate_resize(size(px(680.), px(560.)));
    let issue = item(Provider::Github, "377", "Icons", "2026-10-04T00:00:00Z");
    let key = issue.key.canonical();
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.snapshot.items = Arc::new(vec![issue]);
            view.refresh_rows();
            view.open_dispatch(key, window, cx);
        })
    });
    cx.run_until_parked();
    let card = cx.debug_bounds("orchestrator-overlay-card").unwrap();
    for selector in ["orchestrator-dialog-close", "orchestrator-dialog-dispatch"] {
        let control = cx.debug_bounds(selector).unwrap();
        assert!(
            control.right() <= card.right(),
            "{selector}: {control:?} in {card:?}"
        );
    }
    assert!(card.right() <= px(680.));
}
