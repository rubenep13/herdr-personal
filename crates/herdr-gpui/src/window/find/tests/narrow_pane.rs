//! The find bar over a pane too narrow for it keeps its least width, its
//! count clear of the arrows, and stays over the terminal area.
use super::*;
use gpui::px;

/// The bar open over a pane `columns` wide at the left edge, at `[ui]` 18.
fn narrow(cx: &mut TestAppContext, columns: u16) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    let (view, _peer, cx) = open(cx, &["pane.copy_search"]);
    view.update(cx, |view, cx| {
        view.config.ui.size = 18.;
        let surface = Arc::make_mut(view.live.surface.as_mut().unwrap());
        surface.panes[0].rect.x = 0;
        surface.panes[0].rect.width = columns;
        cx.notify();
    });
    find(&view, cx);
    (view, cx)
}

#[gpui::test]
fn the_find_bar_keeps_its_width_in_a_narrow_pane(cx: &mut TestAppContext) {
    let (view, cx) = narrow(cx, 24);
    let bar = cx.debug_bounds("find-bar").unwrap();
    let least = view.read_with(cx, |view, _| view.view_look().at(260.));
    assert!(bar.size.width >= least - px(0.5), "{bar:?}");
    let count = cx.debug_bounds("find-count").unwrap();
    let older = cx.debug_bounds("find-older").unwrap();
    assert!(count.right() <= older.left(), "{count:?} before {older:?}");
}

#[gpui::test]
fn the_find_bar_stays_over_the_terminals(cx: &mut TestAppContext) {
    let (view, cx) = narrow(cx, 24);
    let bar = cx.debug_bounds("find-bar").unwrap();
    let left = view.read_with(cx, |view, _| view.bounds.left());
    assert!(bar.left() >= left, "{bar:?} from {left:?}");
}

#[gpui::test]
fn a_wide_pane_keeps_the_bar_at_its_width(cx: &mut TestAppContext) {
    let (view, _peer, cx) = open(cx, &["pane.copy_search"]);
    find(&view, cx);
    let bar = cx.debug_bounds("find-bar").unwrap();
    assert!((bar.size.width - px(320.)).abs() < px(0.5), "{bar:?}");
}
