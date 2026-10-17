use super::*;
use crate::{
    config::TabPlacement, icons::AgentIcon, sidebar::layout_tests::full_draw,
    window::tab_strip::Placement,
};
use gpui::{Bounds, Entity, Pixels, TestAppContext, VisualTestContext, px, size};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
}

/// The fixture window on the protocol fixture's one space, `repo`, with
/// `tabs` in it: Claude runs in the first, the rest are terminals.
pub(in crate::window) fn tabbed<'a>(
    cx: &'a mut TestAppContext,
    tabs: usize,
    ui: &str,
    placement: TabPlacement,
) -> (Entity<HerdrWindow>, &'a mut VisualTestContext) {
    let shared = crate::herdr_settings::Settings::parse_text(&format!("[ui]\n{ui}")).unwrap();
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let mut view = fixture_window(window, cx);
        let mut snapshot: herdr_client::protocol::ClientShellSnapshot = serde_json::from_str(
            include_str!("../../../../herdr-protocol/tests/fixtures/endpoint-snapshot-v1.json"),
        )
        .unwrap();
        let first = snapshot.tabs[0].clone();
        snapshot.tabs = (0..tabs)
            .map(|index| {
                let mut tab = first.clone();
                tab.tab_id = format!("t{index}");
                tab.label = format!("tab {index}");
                tab.focused = index == 0;
                tab
            })
            .collect();
        snapshot.focused_tab_id = Some("t0".into());
        snapshot.agents[0].tab_id = "t0".into();
        view.live.snapshot = Some(Arc::new(snapshot));
        view.live.status = crate::state::ConnectionStatus::Connected;
        view.settings.shared = Some(shared);
        view.config.tabs_style.placement = placement;
        view
    });
    cx.simulate_resize(size(px(1200.), px(600.)));
    draw(cx);
    draw(cx);
    (view, cx)
}

fn bounds(cx: &mut VisualTestContext, selector: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("missing {selector}"))
}

#[gpui::test]
fn header_names_the_focused_tab_and_its_place(cx: &mut TestAppContext) {
    let (view, cx) = tabbed(cx, 2, "", TabPlacement::Subheader);
    let place = view.read_with(cx, |view, _| view.header_place()).unwrap();
    assert_eq!(place.title, "tab 0");
    assert_eq!(
        (place.host.as_str(), place.space.as_str()),
        ("Local", "repo")
    );
    assert_eq!(place.agent, Some(AgentIcon::Claude));
    let header = bounds(cx, "titlebar");
    for part in [
        "main-header-tile",
        "main-header-title",
        "main-header-crumbs",
    ] {
        let part = bounds(cx, part);
        assert!(header.contains(&part.center()), "{part:?} {header:?}");
    }
    // A terminal tab names itself, with no agent's mark.
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let mut snapshot = (**view.live.snapshot.as_ref().unwrap()).clone();
            for tab in &mut snapshot.tabs {
                tab.focused = tab.tab_id == "t1";
            }
            snapshot.focused_tab_id = Some("t1".into());
            view.live.snapshot = Some(Arc::new(snapshot));
            cx.notify();
        })
    });
    draw(cx);
    let place = view.read_with(cx, |view, _| view.header_place()).unwrap();
    assert_eq!((place.title.as_str(), place.agent), ("tab 1", None));
}

#[gpui::test]
fn subheader_sits_under_the_header(cx: &mut TestAppContext) {
    let (_, cx) = tabbed(
        cx,
        2,
        "tab_bar_position = 'bottom'",
        TabPlacement::Subheader,
    );
    let header = bounds(cx, "titlebar");
    let strip = bounds(cx, "tab-strip");
    assert_eq!(strip.top(), header.bottom());
    assert_eq!(header.size.height, px(crate::design::HEADER_HEIGHT));
    // The header spans the content column, beside the sidebar's own header.
    let sidebar = bounds(cx, "sidebar-titlebar");
    assert_eq!(header.left(), sidebar.right());
    assert_eq!(header.top(), sidebar.top());
    assert!(sidebar.contains(&bounds(cx, "toggle-sidebar").center()));
}

#[gpui::test]
fn bottom_placement_puts_strips_at_the_foot(cx: &mut TestAppContext) {
    let (_, cx) = tabbed(cx, 2, "", TabPlacement::Bottom);
    let header = bounds(cx, "titlebar");
    let group = bounds(cx, "group");
    let strip = bounds(cx, "tab-strip");
    assert_eq!(strip.bottom(), group.bottom());
    assert!(strip.top() > header.bottom() + px(100.));
}

#[gpui::test]
fn herdr_placement_follows_tab_bar_position(cx: &mut TestAppContext) {
    for (position, placement, expected) in [
        ("top", TabPlacement::Herdr, Placement::Subheader),
        ("bottom", TabPlacement::Herdr, Placement::Bottom),
        ("bottom", TabPlacement::Subheader, Placement::Subheader),
        ("top", TabPlacement::Bottom, Placement::Bottom),
    ] {
        let (view, cx) = tabbed(
            cx,
            2,
            &format!("tab_bar_position = '{position}'"),
            placement,
        );
        assert_eq!(
            view.read_with(cx, |view, _| view.tab_placement()),
            expected,
            "{position} {placement:?}"
        );
    }
}

#[gpui::test]
fn many_tabs_scroll_and_keep_window_controls_inside(cx: &mut TestAppContext) {
    let (_, cx) = tabbed(cx, 30, "", TabPlacement::Subheader);
    cx.simulate_resize(size(px(800.), px(600.)));
    draw(cx);
    draw(cx);
    let avatar = bounds(cx, "titlebar-avatar");
    assert!(avatar.right() <= px(800.), "{avatar:?}");
    let strip = bounds(cx, "tab-strip");
    assert!(strip.right() <= px(800.), "{strip:?}");
    let more = bounds(cx, "tab-actions");
    assert!(more.right() <= px(800.), "{more:?}");
    // The chosen tab keeps its underline, scrolled into view.
    let underline = bounds(cx, "tab-underline-t0");
    assert!(underline.left() >= strip.left() && underline.right() <= strip.right());
}
