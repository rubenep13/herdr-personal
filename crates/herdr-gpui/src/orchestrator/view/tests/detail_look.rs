//! An item's page in the redesign's pieces: its bar, section labels, run
//! cards and the Merge menu.
use super::*;
use crate::orchestrator::{
    Access, Request, Timing,
    service::PullRequestLookup,
    view::{Detail, DetailTab, Look, OrchestratorView, look::sentence},
};
use gpui::{Modifiers, TestAppContext, px, size};
use std::sync::Arc;

fn opened(
    cx: &mut TestAppContext,
    item: Item,
    tab: DetailTab,
    setup: impl FnOnce(&mut OrchestratorView),
) -> (
    tempfile::TempDir,
    gpui::Entity<OrchestratorView>,
    &mut gpui::VisualTestContext,
) {
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
    cx.simulate_resize(size(px(1200.), px(800.)));
    view.update(cx, |view, cx| {
        let key = item.key.canonical();
        view.snapshot.items = Arc::new(vec![item]);
        view.snapshot.access = Some(Access::ReadWrite);
        setup(view);
        view.refresh_rows();
        view.detail = Some(Detail { key, tab });
        view.follow_pull_request();
        cx.notify();
    });
    cx.run_until_parked();
    (dir, view, cx)
}

/// A pull request the fixture's lookup answers, open and mergeable.
fn pull_request() -> Item {
    let mut item = item(
        Provider::Github,
        "8",
        "Improve workspace menus",
        "2026-10-04T00:00:00Z",
    );
    item.url = Some("https://github.com/example/project/pull/8".into());
    item.pull_request = Some(crate::orchestrator::PullRequest {
        number: 8,
        additions: Some(3),
        deletions: Some(1),
        base_ref: "main".into(),
        head_ref: "feature".into(),
        base_sha: "a".into(),
        head_sha: "0123456789abcdef0123456789abcdef01234567".into(),
        head_repository: None,
    });
    item
}

fn looked_up(view: &mut OrchestratorView) {
    view.snapshot.pull_request = Some(PullRequestLookup {
        number: 8,
        result: Arc::new(crate::pull_request::fixture().map(Some)),
    });
}

fn bounds(cx: &mut gpui::VisualTestContext, selector: &'static str) -> gpui::Bounds<gpui::Pixels> {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector}"))
}

#[test]
fn section_labels_are_sentence_case() {
    for (label, sentenced) in [
        ("OVERVIEW", "Overview"),
        ("LATEST RUN", "Latest run"),
        ("EARLIER RUNS", "Earlier runs"),
        ("CHECKS \u{b7} 3", "Checks \u{b7} 3"),
        ("", ""),
    ] {
        assert_eq!(sentence(label), sentenced);
    }
}

#[gpui::test]
fn the_detail_bar_holds_back_crumbs_and_actions(cx: &mut TestAppContext) {
    let (_dir, _view, cx) = opened(cx, pull_request(), DetailTab::Description, looked_up);
    let bar = bounds(cx, "orchestrator-detail-bar");
    assert_eq!(bar.size.height, px(40.));
    let mut left = bar.left();
    for selector in [
        "orchestrator-back",
        "orchestrator-crumb-list",
        "orchestrator-detail-merge",
        "orchestrator-detail-review",
        "orchestrator-detail-web",
    ] {
        let control = bounds(cx, selector);
        assert!(control.left() >= left, "{selector} is out of order");
        assert!(control.right() <= bar.right(), "{selector}");
        left = control.left();
    }
    assert_eq!(
        bounds(cx, "orchestrator-detail-review").size.height,
        px(26.)
    );
    assert_eq!(bounds(cx, "orchestrator-back").size.height, px(28.));
    assert!(bounds(cx, "orchestrator-detail-title").top() >= bar.bottom());
}

#[gpui::test]
fn run_cards_keep_their_buttons(cx: &mut TestAppContext) {
    let issue = item(Provider::Github, "5", "Icons", "2026-10-04T00:00:00Z");
    let running = run("r1", &issue, RunState::Running, "2026-10-04T01:00:00Z");
    let (_dir, _view, cx) = opened(cx, issue, DetailTab::Agent, |view| {
        view.snapshot.sessions = Arc::new(vec![session(&running, "p1")]);
        view.live = Arc::new(vec![live(&running, "p1", AgentStatus::Working)]);
        view.recorded_runs = Arc::new(vec![running]);
    });
    let card = bounds(cx, "orchestrator-card-0");
    let open = bounds(cx, "orchestrator-card-open-0");
    assert_eq!(open.size.height, px(26.));
    assert!(card.contains(&open.origin) && card.contains(&open.bottom_right()));
}

#[gpui::test]
fn the_merge_menu_is_a_menu(cx: &mut TestAppContext) {
    let (_dir, _view, cx) = opened(cx, pull_request(), DetailTab::Description, looked_up);
    let merge = bounds(cx, "orchestrator-detail-merge");
    cx.simulate_click(merge.center(), Modifiers::none());
    cx.run_until_parked();
    let menu = bounds(cx, "orchestrator-merge-menu");
    let squash = bounds(cx, "orchestrator-merge-SQUASH");
    assert!(menu.contains(&squash.center()));
    assert_eq!(squash.size.height, px(32.));
}
