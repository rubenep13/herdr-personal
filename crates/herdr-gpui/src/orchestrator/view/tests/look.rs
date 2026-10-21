//! The inbox in the redesign's pieces: its bar, filters, rows and notices.
use super::*;
use crate::orchestrator::{
    Access, Request, SourceStatus, SyncState, Timing,
    view::{Look, OrchestratorView, rows::Tab},
};
use gpui::{Modifiers, TestAppContext, px, size};
use std::sync::Arc;

fn issues(count: usize) -> Vec<Item> {
    (0..count)
        .map(|index| {
            item(
                Provider::Github,
                &index.to_string(),
                &format!("Issue {index}"),
                "2026-10-04T00:00:00Z",
            )
        })
        .collect()
}

/// An inbox listing `items`, its first selected, in a window 1200 wide.
fn inbox(
    cx: &mut TestAppContext,
    items: Vec<Item>,
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
    let look = Look {
        indicators: crate::sidebar::Indicators::new(None, None, false, &theme),
        theme,
        ui: config.ui.clone(),
        mono: config.terminal.clone(),
        light: false,
    };
    let (view, cx) = cx.add_window_view(|_, cx| OrchestratorView::new(request, look, cx));
    cx.simulate_resize(size(px(1200.), px(700.)));
    view.update(cx, |view, cx| {
        let first = items.first().map(|item| item.key.canonical());
        view.snapshot.items = Arc::new(items);
        view.snapshot.access = Some(Access::ReadWrite);
        view.tab = Tab::Issues;
        view.selected = first;
        view.refresh_rows();
        cx.notify();
    });
    cx.run_until_parked();
    (dir, view, cx)
}

fn bounds(cx: &mut gpui::VisualTestContext, selector: &'static str) -> gpui::Bounds<gpui::Pixels> {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector}"))
}

#[gpui::test]
fn the_inbox_wears_the_view_bar(cx: &mut TestAppContext) {
    let (_dir, _view, cx) = inbox(cx, issues(3));
    let bar = bounds(cx, "orchestrator-bar");
    assert_eq!(bar.size.height, px(40.));
    let dispatch = bounds(cx, "orchestrator-dispatch");
    assert_eq!(dispatch.size.height, px(26.));
    for selector in [
        "orchestrator-refresh",
        "orchestrator-window",
        "orchestrator-preview",
    ] {
        let button = bounds(cx, selector);
        assert_eq!(button.size.height, px(28.), "{selector}");
        assert!(
            button.right() <= dispatch.left(),
            "{selector} before dispatch"
        );
    }
    assert!(dispatch.right() <= bar.right());
}

#[gpui::test]
fn filters_are_pills(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    let pill = bounds(cx, "orchestrator-has-run");
    assert_eq!(pill.size.height, px(26.));
    assert_eq!(bounds(cx, "orchestrator-filters").size.height, px(46.));
    cx.simulate_click(pill.center(), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |view, _| assert!(view.filters.has_run));
    assert!(cx.debug_bounds("orchestrator-has-run-check").is_some());
}

#[gpui::test]
fn rows_are_inset_and_select_on_click(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    let list = bounds(cx, "orchestrator-list");
    let row = bounds(cx, "orchestrator-issue-1");
    let inset = row.left() - list.left();
    assert!(inset >= px(8.) && inset <= px(9.), "{inset:?}");
    assert_eq!(row.size.height, px(32.));
    cx.simulate_click(row.center(), Modifiers::none());
    cx.run_until_parked();
    let picked = view.read_with(cx, |view, _| view.selected.clone());
    let second = view.read_with(cx, |view, _| view.snapshot.items[1].key.canonical());
    assert_eq!(picked, Some(second));
}

#[gpui::test]
fn a_sync_failure_shows_the_whole_error(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    let before = bounds(cx, "orchestrator-bar");
    let failure = crate::orchestrator::Error::RemoteUrl("x".repeat(300));
    view.update(cx, |view, cx| {
        view.snapshot.sources = vec![SourceStatus {
            key: source(Provider::Github),
            state: SyncState::Failed(Arc::new(failure)),
        }];
        cx.notify();
    });
    cx.run_until_parked();
    assert_eq!(bounds(cx, "orchestrator-bar"), before);
    let banner = bounds(cx, "orchestrator-banner-0");
    let filters = bounds(cx, "orchestrator-filters");
    let list = bounds(cx, "orchestrator-list");
    assert!(banner.top() >= filters.bottom() && banner.bottom() <= list.top());
    // The whole text wraps rather than ending in an ellipsis.
    assert!(banner.size.height > px(40.), "{banner:?}");
    assert!(cx.debug_bounds("orchestrator-banner-0-icon").is_some());
}

#[gpui::test]
fn thousands_of_issues_paint_a_screenful(cx: &mut TestAppContext) {
    let drawn = |cx: &mut gpui::VisualTestContext, count: usize| {
        (0..count)
            .filter(|index| {
                cx.debug_bounds(format!("orchestrator-issue-{index}").leak())
                    .is_some()
            })
            .count()
    };
    let few = {
        let (_dir, _view, cx) = inbox(cx, issues(100));
        drawn(cx, 100)
    };
    let (_dir, _view, cx) = inbox(cx, issues(2000));
    let many = drawn(cx, 2000);
    assert!(few > 0 && few < 100, "{few}");
    assert_eq!(few, many);
}

#[gpui::test]
fn a_narrow_inbox_keeps_its_buttons(cx: &mut TestAppContext) {
    let (_dir, view, cx) = inbox(cx, issues(3));
    view.update(cx, |view, cx| {
        view.look.ui.size = 18.;
        cx.notify();
    });
    cx.simulate_resize(size(px(680.), px(560.)));
    cx.run_until_parked();
    let bar = bounds(cx, "orchestrator-bar");
    assert!((bar.size.height - px(60.)).abs() < px(1.));
    let dispatch = bounds(cx, "orchestrator-dispatch");
    assert!(dispatch.right() <= bar.right(), "{dispatch:?} in {bar:?}");
    let filters = bounds(cx, "orchestrator-filters");
    let sort = bounds(cx, "orchestrator-sort");
    assert!(sort.right() <= filters.right(), "{sort:?} in {filters:?}");
}
