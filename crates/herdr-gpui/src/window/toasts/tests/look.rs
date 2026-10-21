//! Toasts in the redesigned look: the menus' panel, the status dot in the
//! colour the sidebar gives that state, and the host's own colour.

use super::*;
use herdr_client::protocol::{AgentStatus, SemanticNotificationKind};

/// A toast preview drawn in an 800 × 600 window, with the selectors of its
/// card, host dot and cross.
fn preview(
    view: &gpui::Entity<crate::HerdrWindow>,
    cx: &mut gpui::VisualTestContext,
) -> (&'static str, &'static str, &'static str) {
    cx.simulate_resize(size(px(800.), px(600.)));
    let (endpoint, id) = view.update(cx, |view, cx| {
        view.config.notifications.delay_seconds = 3600;
        view.show_toast_preview(SemanticNotificationKind::NeedsAttention, cx);
        let endpoint = view.endpoints[0].id.clone();
        let id = view.endpoints[0].toasts.entries.back().unwrap().0;
        (endpoint, id)
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (
        format!("toast-{endpoint}-{id}").leak(),
        format!("toast-host-dot-{endpoint}-{id}").leak(),
        format!("toast-dismiss-{endpoint}-{id}").leak(),
    )
}

#[gpui::test]
fn a_toast_dot_says_what_the_sidebar_says(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    for latte in [false, true] {
        view.update(cx, |view, _| {
            if latte {
                view.theme = crate::config::Theme::builtin("Catppuccin Latte").unwrap();
                view.theme_light = true;
            }
            assert_eq!(
                view.toast_dot(SemanticNotificationKind::NeedsAttention),
                view.status_color(AgentStatus::Blocked)
            );
            assert_eq!(
                view.toast_dot(SemanticNotificationKind::Finished),
                view.status_color(AgentStatus::Done)
            );
            assert_eq!(
                view.toast_dot(SemanticNotificationKind::Custom),
                view.theme.ink(view.theme.primary())
            );
        });
    }
}

#[gpui::test]
fn the_host_dot_follows_the_host_colour(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    let label = view.update(cx, |view, _| view.endpoints[0].label.clone());
    let (_, dot, _) = preview(&view, cx);
    assert!(
        cx.debug_bounds(dot).is_none(),
        "an untinted host has no dot"
    );
    view.update(cx, |view, cx| {
        view.config
            .sidebar_style
            .hosts
            .insert(label.clone(), 0xcba6f7);
        assert_eq!(view.host_dot(&label), Some(view.theme.ink(0xcba6f7)));
        assert_eq!(view.host_dot("Mac"), None);
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        cx.debug_bounds(dot).is_some(),
        "a tinted host shows its dot"
    );
}

#[gpui::test]
fn a_long_host_truncates_and_keeps_the_cross(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        view.endpoints[0].label = "a-very-long-host-label-".repeat(4);
    });
    let (card, _, cross) = preview(&view, cx);
    let card = cx.debug_bounds(card).unwrap();
    let cross = cx.debug_bounds(cross).unwrap();
    assert!(
        card.contains(&cross.origin) && cross.right() <= card.right(),
        "{cross:?} in {card:?}"
    );
    assert_eq!(cross.size.width, px(24.));
}
