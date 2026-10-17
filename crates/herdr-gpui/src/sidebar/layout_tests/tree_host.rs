use super::tree_layout::{draw, tree_window};
use super::*;
use crate::{sidebar::tree_host::host_dot, state::ConnectionStatus, usage::Host};

fn seed_load(view: &Entity<HerdrWindow>, cx: &mut gpui::VisualTestContext, host: Host) {
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.system_load.seed(host, 12., (8, 16));
            cx.notify();
        })
    });
    draw(cx);
}

/// The texts one fresh frame paints.
fn drawn_texts(cx: &mut gpui::VisualTestContext) -> Vec<String> {
    cx.update(|window, cx| {
        cx.default_global::<TextProbes>().0.clear();
        full_draw(window, cx).clear(cx);
        cx.global::<TextProbes>().0.keys().cloned().collect()
    })
}

#[gpui::test]
fn tree_host_shows_name_kind_dot_and_load(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, true);
    seed_load(&view, cx, Host::Local);
    let row = cx.debug_bounds("host-local").unwrap();
    let parts = [
        "host-label-local",
        "host-kind-local",
        "host-dot-local",
        "host-load-local",
    ]
    .map(|selector| {
        cx.debug_bounds(selector)
            .unwrap_or_else(|| panic!("{selector}"))
    });
    for (pair, part) in parts.windows(2).zip(&parts[1..]) {
        assert!(
            pair[0].right() <= pair[1].left(),
            "{part:?} follows {pair:?}"
        );
    }
    for part in parts {
        assert!(part.right() <= row.right() && part.top() >= row.top());
    }
    assert_eq!(row.size.height, px(crate::design::ROW_HEIGHT));
    let texts = drawn_texts(cx);
    for text in ["Local", "12% CPU", "50% mem"] {
        assert!(texts.iter().any(|drawn| drawn == text), "{text}: {texts:?}");
    }
    // Hover actions take no room until the row is pointed at.
    let hidden = |cx: &mut gpui::VisualTestContext, selector| {
        cx.debug_bounds(selector)
            .is_none_or(|bounds| bounds.size.width == px(0.))
    };
    assert!(hidden(cx, "host-new-space-local"));
    assert!(hidden(cx, "host-fold-local"));
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.config.show_system_load = false;
            cx.notify();
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("host-load-local").is_none());
}

#[gpui::test]
fn disconnected_host_has_no_load_and_a_muted_dot(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, true);
    let target = view.read_with(cx, |view, _| view.endpoints[1].connection.target.clone());
    let host = Host::of(&target).unwrap();
    seed_load(&view, cx, host);
    assert!(cx.debug_bounds("host-load-ssh:test").is_some());
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.endpoints[1].live.status = ConnectionStatus::Disconnected;
            cx.notify();
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("host-dot-ssh:test").is_some());
    assert!(cx.debug_bounds("host-load-ssh:test").is_none());
    // Its last known rows stay listed.
    assert!(cx.debug_bounds("workspace-ssh:test-w0").is_some());
    view.read_with(cx, |view, _| {
        let theme = &view.theme;
        assert_eq!(
            host_dot(&ConnectionStatus::Disconnected, theme),
            theme.muted
        );
        assert_eq!(
            host_dot(&ConnectionStatus::Connected, theme),
            crate::menu::online(theme)
        );
    });
}

#[gpui::test]
fn new_space_button_targets_its_host(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, true);
    // The fixture's daemon must let the window switch hosts at all.
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            view.live.supports_surface = true;
            view.endpoints[0].live.supports_surface = true;
        })
    });
    assert_eq!(view.read_with(cx, |view, _| view.selected_endpoint), 0);
    let row = cx.debug_bounds("host-ssh:test").unwrap();
    cx.simulate_mouse_move(row.center(), None, Modifiers::none());
    draw(cx);
    let button = cx
        .debug_bounds("host-new-space-ssh:test")
        .expect("the hovered host shows its new space button");
    let fold = cx.debug_bounds("host-fold-ssh:test").unwrap();
    assert!(button.right() <= row.right() && fold.right() <= row.right());
    cx.simulate_click(button.center(), Modifiers::none());
    draw(cx);
    assert_eq!(view.read_with(cx, |view, _| view.selected_endpoint), 1);
}
