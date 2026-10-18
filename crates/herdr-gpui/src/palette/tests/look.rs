use super::*;

/// The status fixture's palette, open on `filter` in a window wide enough
/// for Go To's preview.
fn open<'a>(
    filter: Filter,
    cx: &'a mut TestAppContext,
) -> (Entity<HerdrWindow>, &'a mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
        agent_filter::status_fixture(window, cx)
    });
    cx.simulate_resize(size(px(1400.), px(900.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| view.open_palette(filter, window, cx));
        window.draw(cx).clear(cx);
    });
    // The list lays its rows out on the frame after it is sized.
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (view, cx)
}

fn redraw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

/// The listed rows' indexes whose entry `matches`.
fn rows(
    view: &Entity<HerdrWindow>,
    cx: &mut VisualTestContext,
    matches: impl Fn(&Entry, usize) -> bool,
) -> Vec<usize> {
    view.read_with(cx, |view, _| {
        let palette = view.menu.palette.as_ref().unwrap();
        palette
            .filtered
            .iter()
            .enumerate()
            .filter(|(_, hit)| matches(&palette.entries[hit.index], hit.depth))
            .map(|(index, _)| index)
            .collect()
    })
}

fn bounds(cx: &mut VisualTestContext, selector: String) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(selector.leak())
}

#[gpui::test]
fn the_query_leads_with_a_search_icon(cx: &mut TestAppContext) {
    let (_, cx) = open(Filter::All, cx);
    let icon = cx.debug_bounds("palette-search-icon").unwrap();
    assert_eq!(icon.size, size(px(18.), px(18.)));
}

#[gpui::test]
fn filters_are_underlined_tabs(cx: &mut TestAppContext) {
    let (_, cx) = open(Filter::All, cx);
    let filters = cx.debug_bounds("palette-filters").unwrap();
    let underline = cx.debug_bounds("palette-filter-underline").unwrap();
    assert_eq!(underline.size.height, px(2.));
    assert_eq!(underline.bottom(), filters.bottom());
    let all = cx.debug_bounds("palette-filter-all").unwrap();
    assert!(underline.left() > all.left() && underline.right() < all.right());
    cx.simulate_keystrokes("ctrl-tab");
    redraw(cx);
    let moved = cx.debug_bounds("palette-filter-underline").unwrap();
    let navigation = cx.debug_bounds("palette-filter-navigation").unwrap();
    assert!(moved.left() > navigation.left() && moved.right() < navigation.right());
}

#[gpui::test]
fn rows_are_list_rows_with_a_tile(cx: &mut TestAppContext) {
    let (_, cx) = open(Filter::Commands, cx);
    let panel = cx.debug_bounds("menu-panel").unwrap();
    let row = cx.debug_bounds("palette-row-0").unwrap();
    assert_eq!(row.size.height, px(44.));
    // The panel's 1 px border and the list's 6 px inset.
    assert_eq!(row.left(), panel.left() + px(7.));
    let tile = cx.debug_bounds("palette-tile-0").unwrap();
    assert_eq!(tile.size, size(px(24.), px(24.)));
}

#[gpui::test]
fn agent_rows_show_the_tree_badge_and_state_pill(cx: &mut TestAppContext) {
    let (view, cx) = open(Filter::Navigation, cx);
    let agent = rows(&view, cx, |entry, _| entry.status.is_some())[0];
    let state = bounds(cx, format!("palette-state-{agent}")).unwrap();
    assert_eq!(state.size, size(px(14.), px(14.)));
    assert!(bounds(cx, format!("palette-pill-{agent}")).is_some());
    let workspace = rows(&view, cx, |entry, _| {
        matches!(
            entry.action,
            Action::Go {
                target: NavigationTarget::Workspace(_),
                ..
            }
        )
    })[0];
    assert!(bounds(cx, format!("palette-state-{workspace}")).is_none());
    assert!(bounds(cx, format!("palette-badge-{workspace}")).is_some());
}

#[gpui::test]
fn nested_rows_hang_on_a_guide_only_without_a_query(cx: &mut TestAppContext) {
    let (view, cx) = open(Filter::Navigation, cx);
    let nested = rows(&view, cx, |_, depth| depth == 1)[0];
    let parent = rows(&view, cx, |_, depth| depth == 0)
        .into_iter()
        .rfind(|index| *index < nested)
        .unwrap();
    assert!(bounds(cx, format!("palette-guide-{nested}")).is_some());
    assert!(bounds(cx, format!("palette-guide-{parent}")).is_none());
    let inner = bounds(cx, format!("palette-tile-{nested}")).unwrap();
    let outer = bounds(cx, format!("palette-tile-{parent}")).unwrap();
    assert_eq!(inner.left() - outer.left(), px(18.));

    cx.simulate_input("w");
    redraw(cx);
    let shown = view.read_with(cx, |view, _| {
        view.menu.palette.as_ref().unwrap().filtered.len()
    });
    assert!(shown > 0);
    for index in 0..shown.min(10) {
        assert!(bounds(cx, format!("palette-guide-{index}")).is_none());
    }
}

/// Every key of every hint is drawn as a key, whichever hints the filter
/// shows.
#[gpui::test]
fn the_footer_draws_every_hint_key_as_a_key(cx: &mut TestAppContext) {
    for (filter, hints) in [
        (Filter::All, hints::HINTS),
        (Filter::Navigation, hints::GO_TO_HINTS),
    ] {
        let (_, cx) = open(filter, cx);
        let expected: usize = hints.iter().map(|hint| hint.keys.len()).sum();
        let drawn = (0..)
            .take_while(|index| bounds(cx, format!("palette-key-{index}")).is_some())
            .count();
        assert_eq!(drawn, expected, "{filter:?}");
    }
}
