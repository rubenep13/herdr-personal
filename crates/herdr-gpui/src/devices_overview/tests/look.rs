use super::*;
use gpui::{px, size};
use herdr_client::protocol::AgentStatus;

fn bounds(cx: &mut VisualTestContext, selector: &'static str) -> gpui::Bounds<gpui::Pixels> {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector}"))
}

#[gpui::test]
fn the_overview_wears_the_view_bar(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1400.), px(900.)));
    open(&view, cx);
    let bar = bounds(cx, "devices-bar");
    let add = bounds(cx, "devices-add");
    let field = bounds(cx, "devices-search");
    assert_eq!(bar.size.height, px(40.));
    assert_eq!(add.size.height, px(26.));
    assert!(add.left() >= field.right(), "{add:?} after {field:?}");
    assert!(add.right() <= bar.right());
    let tab = bounds(cx, "devices-tab");
    assert_eq!(bar.top(), tab.top(), "the bar heads the tab");
}

#[gpui::test]
fn totals_are_four_cards_in_a_row(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1400.), px(900.)));
    open(&view, cx);
    let first = bounds(cx, "devices-total-0");
    for selector in ["devices-total-1", "devices-total-2", "devices-total-3"] {
        let card = bounds(cx, selector);
        assert_eq!(card.top(), first.top(), "{selector}");
        assert!(
            (card.size.width - first.size.width).abs() <= px(2.),
            "{selector}: {card:?} beside {first:?}"
        );
        assert!(card.left() > first.left());
    }
    let activity = bounds(cx, "devices-activity");
    assert!(activity.top() > first.bottom(), "the totals come first");
}

#[gpui::test]
fn device_rows_are_inset_and_two_lines_tall(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(1400.), px(900.)));
    open(&view, cx);
    let table = bounds(cx, "devices-table");
    let row = bounds(cx, "devices-row-local");
    assert_eq!(row.size.height, px(44.));
    let inset = row.left() - table.left();
    assert!(inset >= px(8.) && inset <= px(9.), "{inset:?}");
    assert!(table.right() - row.right() >= px(8.));
}

#[gpui::test]
fn dots_take_the_users_status_colours(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx);
    view.update(cx, |view, _| {
        view.settings.shared = Some(
            crate::herdr_settings::Settings::parse_text("[theme.custom]\nyellow='#ff8800'\n")
                .unwrap(),
        );
    });
    cx.update(|_, cx| {
        let view = view.read(cx);
        let look = view.overview_look(cx);
        assert_eq!(look.status(AgentStatus::Working), view.theme.ink(0xff8800));
    });
}

#[gpui::test]
fn a_narrow_overview_keeps_add_device(cx: &mut gpui::TestAppContext) {
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(680.), px(560.)));
    view.update(cx, |view, _| view.config.ui.size = 18.);
    open(&view, cx);
    let bar = bounds(cx, "devices-bar");
    let add = bounds(cx, "devices-add");
    let field = bounds(cx, "devices-search");
    assert!(add.right() <= bar.right(), "{add:?} in {bar:?}");
    assert!(
        field.size.width < px(240. * 18. / 12.),
        "the field gave way"
    );
    assert!((bar.size.height - px(60.)).abs() < px(1.));
}
