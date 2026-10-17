use super::*;
use crate::config::{TabContent, TabPlacement};
use crate::window::tests::header::tabbed;

fn set_content(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext, content: TabContent) {
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.config.tabs_style.content = content;
            cx.notify();
        })
    });
    draw(cx);
}

#[gpui::test]
fn active_tab_is_underlined_in_the_accent(cx: &mut TestAppContext) {
    let (view, cx) = tabbed(cx, 2, "", TabPlacement::Subheader);
    let strip = cx.debug_bounds("tab-strip").unwrap();
    let underline = cx.debug_bounds("tab-underline-t0").unwrap();
    assert!(cx.debug_bounds("tab-underline-t1").is_none());
    let tab = cx.debug_bounds("tab-t0").unwrap();
    assert_eq!(underline.bottom(), strip.bottom());
    assert_eq!(underline.size.height, px(2.));
    assert!(underline.left() > tab.left() && underline.right() < tab.right());
    view.read_with(cx, |view, _| {
        let group = view.active_group().unwrap();
        assert_eq!(view.tab_underline(group), view.theme.primary());
    });
    // At the foot the line runs along the strip's top edge instead.
    let (_, cx) = tabbed(&mut cx.cx, 2, "", TabPlacement::Bottom);
    let strip = cx.debug_bounds("tab-strip").unwrap();
    let underline = cx.debug_bounds("tab-underline-t0").unwrap();
    assert_eq!(underline.top(), strip.top());
}

#[gpui::test]
fn tab_content_option_adds_logo_then_status(cx: &mut TestAppContext) {
    let (view, cx) = tabbed(cx, 2, "", TabPlacement::Subheader);
    let name_only = cx.debug_bounds("tab-t0").unwrap();
    assert!(cx.debug_bounds("tab-logo-t0").is_none());
    assert!(cx.debug_bounds("tab-status-t0").is_none());
    set_content(&view, cx, TabContent::Agent);
    assert!(cx.debug_bounds("tab-logo-t0").is_some());
    assert!(cx.debug_bounds("tab-status-t0").is_none());
    set_content(&view, cx, TabContent::Status);
    let logo = cx.debug_bounds("tab-logo-t0").unwrap();
    let status = cx.debug_bounds("tab-status-t0").unwrap();
    let tab = cx.debug_bounds("tab-t0").unwrap();
    assert!(tab.size.width > name_only.size.width);
    for part in [logo, status] {
        assert!(part.left() >= tab.left() && part.right() <= tab.right());
    }
}

#[gpui::test]
fn agentless_tab_never_shows_status(cx: &mut TestAppContext) {
    let (view, cx) = tabbed(cx, 2, "", TabPlacement::Subheader);
    set_content(&view, cx, TabContent::Status);
    assert!(cx.debug_bounds("tab-logo-t1").is_some());
    assert!(cx.debug_bounds("tab-status-t1").is_none());
}
