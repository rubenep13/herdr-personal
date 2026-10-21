//! The usage panel in the menus' panel: its heading, rows and tabs.
use super::*;
use crate::sidebar::layout_tests::{fixture_window, full_draw};
use gpui::{TestAppContext, point, px, size};

fn readings(ids: &[&str]) -> Vec<Reading> {
    ids.iter()
        .map(|id| Reading {
            provider: provider(id),
            report: Some(report(provider(id), 40.)),
            error: None,
            access: None,
        })
        .collect()
}

fn opened<'a>(
    cx: &'a mut TestAppContext,
    ids: &'static [&'static str],
) -> (
    gpui::Entity<crate::HerdrWindow>,
    &'a mut gpui::VisualTestContext,
) {
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.status = crate::state::ConnectionStatus::Connected;
        view.config.usage.show = true;
        view.usage.host = Some(super::super::Host::Local);
        view.usage.entries.insert(
            super::super::Host::Local,
            super::super::Entry {
                readings: readings(ids),
                ..Default::default()
            },
        );
        view
    });
    cx.simulate_resize(size(px(1400.), px(900.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_usage(provider(ids[0]), point(px(900.), px(870.)), window, cx)
        })
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    (view, cx)
}

#[gpui::test]
fn the_usage_panel_has_a_menu_header(cx: &mut TestAppContext) {
    let (_view, cx) = opened(cx, &["claude"]);
    let panel = cx.debug_bounds("usage-panel").unwrap();
    let header = cx.debug_bounds("usage-panel-header").unwrap();
    assert!(header.top() >= panel.top() && header.top() - panel.top() < px(16.));
    assert!(cx.debug_bounds("usage-panel-header-icon").is_some());
    let refresh = cx.debug_bounds("usage-refresh-row").unwrap();
    assert_eq!(refresh.size.height, px(32.));
}

#[gpui::test]
fn provider_tabs_stay_on_the_bottom_edge(cx: &mut TestAppContext) {
    let (view, cx) = opened(cx, &["claude", "codex"]);
    let refresh = cx.debug_bounds("usage-refresh-row").unwrap();
    let first = cx.debug_bounds("usage-tab-claude").unwrap();
    assert!(first.top() >= refresh.bottom());
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.menu.page = Some(crate::menu::Page::Usage(provider("codex")));
            cx.notify();
        })
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let after = cx.debug_bounds("usage-tab-claude").unwrap();
    assert!((after.bottom() - first.bottom()).abs() <= px(1.));
}
