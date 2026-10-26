/// The reconnect card is the menus' panel with its tile; a long reason
/// wraps inside it and Reconnect now stays in reach.
#[gpui::test]
fn the_reconnect_card_keeps_its_button_inside(cx: &mut gpui::TestAppContext) {
    let peer = crate::window::MockPeer::advertising(&[]);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        peer.prepare(&mut view, cx);
        let selected = view.selected_endpoint;
        view.endpoints[selected].outage = Some("connection reset by peer ".repeat(12));
        view
    });
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    assert!(view.read_with(cx, |view, _| view.reconnecting()));
    let card = cx.debug_bounds("reconnecting").unwrap();
    let button = cx.debug_bounds("reconnect-now").unwrap();
    assert_eq!(button.size.height, gpui::px(26.));
    assert!(card.contains(&button.origin) && card.contains(&button.bottom_right()));
    assert!(cx.debug_bounds("reconnecting-tile").is_some());
}
