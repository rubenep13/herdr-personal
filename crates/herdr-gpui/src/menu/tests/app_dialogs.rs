//! The app's own dialogs in the redesigned look: each has its cross in the
//! heading, and the button that only closes it stays last in the footer.

use super::*;
use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};

fn open(
    page: Page,
    cx: &mut TestAppContext,
) -> (gpui::Entity<crate::HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(900.), px(700.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            if page == Page::About {
                view.open_about(window, cx);
            } else {
                view.open_menu(window, cx);
                view.menu.version_notice = Some(VersionNotice {
                    host: None,
                    mismatch: herdr_client::VersionMismatch::DaemonOutdated {
                        server_version: Some("0.8.2".into()),
                    },
                });
                view.menu.page = Some(page);
            }
        });
        window.draw(cx).clear(cx);
    });
    (view, cx)
}

#[gpui::test]
fn every_app_dialog_has_a_cross_in_its_header(cx: &mut TestAppContext) {
    for (page, cross) in [
        (Page::About, "about-dismiss"),
        (Page::Update, "release-notes-dismiss"),
        (Page::AppUpdate, "app-update-close"),
        (Page::VersionMismatch, "version-mismatch-cross"),
        (Page::AgentSkill, "agent-skill-dismiss"),
    ] {
        let (view, cx) = open(page, cx);
        let panel = cx.debug_bounds("menu-panel").unwrap();
        let bounds = cx.debug_bounds(cross).unwrap();
        assert!(
            bounds.right() >= panel.right() - px(12.) && bounds.top() <= panel.top() + px(52.),
            "{page:?}: {bounds:?} in {panel:?}"
        );
        assert_eq!(bounds.size.width, px(28.), "{page:?}");
        cx.simulate_click(bounds.center(), Modifiers::default());
        assert!(
            view.read_with(cx, |view, _| view.menu.page.is_none()),
            "{page:?}"
        );
    }
}

#[gpui::test]
fn about_keeps_its_ok_button_last(cx: &mut TestAppContext) {
    let (_, cx) = open(Page::About, cx);
    let footer = cx.debug_bounds("about-footer").unwrap();
    let ok = cx.debug_bounds("about-close").unwrap();
    assert!(footer.contains(&ok.center()), "{ok:?} in {footer:?}");
    assert!(
        ok.right() >= footer.right() - px(12.),
        "{ok:?} in {footer:?}"
    );
    assert_eq!(ok.size.height, px(30.));
}

#[gpui::test]
fn the_update_preview_reads_as_a_note(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(900.), px(700.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_app_update(false, window, cx);
            view.update_preview = Some(crate::updater::State::Idle);
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert!(cx.debug_bounds("app-update-preview").is_some());
    assert!(cx.debug_bounds("app-update-preview-note-icon").is_some());
}
