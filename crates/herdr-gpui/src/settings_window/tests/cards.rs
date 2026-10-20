//! Each section's rows sit together in one card under its title.
use super::*;
use crate::herdr_settings::RemotePaneHistory;
use herdr_client::ConnectTarget;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

fn show(view: &Entity<SettingsWindow>, section: Section, cx: &mut VisualTestContext) {
    view.update(cx, |view, cx| {
        view.section = section;
        cx.notify();
    });
    draw(cx);
}

fn inside(cx: &mut VisualTestContext, inner: &'static str, outer: &'static str) {
    let row = cx
        .debug_bounds(inner)
        .unwrap_or_else(|| panic!("missing {inner}"));
    let card = cx
        .debug_bounds(outer)
        .unwrap_or_else(|| panic!("missing {outer}"));
    assert!(
        row.left() >= card.left()
            && row.right() <= card.right()
            && row.top() >= card.top()
            && row.bottom() <= card.bottom(),
        "{inner} {row:?} outside {outer} {card:?}"
    );
}

#[gpui::test]
fn rows_sit_in_their_sections_card(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(1200.), px(3000.)));
    for (section, row, card) in [
        (Section::Sidebar, "sidebar-layout", "settings-card-layout"),
        (Section::Sidebar, "sidebar-indent", "settings-card-spacing"),
        (Section::Tabs, "tabs-confirm-tab", "settings-card-closing"),
        (Section::General, "general-ports", "settings-card-interface"),
        (
            Section::Notifications,
            "notif-delay",
            "settings-card-in-app-toasts",
        ),
        (Section::StatusBar, "status-usage", "settings-card-items"),
        (
            Section::Configuration,
            "config-reload",
            "settings-card-files",
        ),
        (
            Section::Theme,
            "theme-padding-left",
            "settings-card-terminal-padding",
        ),
    ] {
        show(&view, section, cx);
        inside(cx, row, card);
    }
}

#[gpui::test]
fn session_restore_hosts_are_dependent_rows(cx: &mut TestAppContext) {
    let source = cx.add_window(crate::sidebar::layout_tests::fixture_window);
    let weak = cx.update(|cx| source.update(cx, |_, _, cx| cx.weak_entity()).unwrap());
    let (view, cx) = cx.add_window_view(move |_, cx| {
        let mut view = SettingsWindow::new(weak, cx);
        view.section = Section::Tabs;
        view.remote_history.read = Arc::new(|_, _| {
            Ok(RemotePaneHistory::parse_text(Some(
                "[experimental]\npane_history = true\n".into(),
            ))?)
        });
        view
    });
    cx.update(|_, cx| {
        source
            .update(cx, |window, _, cx| {
                let endpoint = &mut window.endpoints[0];
                endpoint.connection.target = ConnectTarget::Ssh {
                    target: "box".into(),
                    session: "default".into(),
                };
                endpoint.live.status = crate::state::ConnectionStatus::Connected;
                cx.notify();
            })
            .unwrap()
    });
    view.update(cx, |view, cx| view.sync_remote_history(false, cx));
    cx.run_until_parked();
    cx.simulate_resize(size(px(1200.), px(3000.)));
    draw(cx);
    assert!(cx.debug_bounds("tabs-pane-history-host-0").is_some());
    assert!(
        cx.debug_bounds("tabs-pane-history-host-0-sub").is_some(),
        "a host's row depends on Restore scrollback"
    );
    inside(
        cx,
        "tabs-pane-history-host-0",
        "settings-card-session-restore",
    );
}

#[gpui::test]
fn narrow_window_keeps_controls_inside_the_body(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(680.), px(560.)));
    show(&view, Section::Sidebar, cx);
    let body = cx.debug_bounds("settings-body").unwrap();
    let control = cx.debug_bounds("sidebar-select-control").unwrap();
    assert!(
        control.right() <= body.right() - px(36.),
        "{control:?} in {body:?}"
    );
    let row = cx.debug_bounds("sidebar-select").unwrap();
    assert!(
        row.right() <= body.right() && control.bottom() <= row.bottom(),
        "the row wraps its description instead of overflowing: {row:?}"
    );
}
