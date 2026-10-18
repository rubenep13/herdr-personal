use super::*;

/// The device picker heads its list with a section and its rows take the
/// menu rows' inset and padding.
#[gpui::test]
fn device_rows_sit_inside_the_panel_inset(cx: &mut TestAppContext) {
    let (_view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.run_until_parked();
    draw(cx);
    click(cx, "device-picker");
    draw(cx);
    let panel = bounds(cx, "menu-panel");
    let row = bounds(cx, "device-row-0");
    assert!(row.left() >= panel.left() + px(7.), "{row:?} {panel:?}");
    let icon = bounds(cx, "device-row-0-icon");
    assert_eq!(icon.left() - row.left(), px(10.));
    assert!(cx.debug_bounds("devices-heading").is_some());
}

#[gpui::test]
fn session_rows_keep_their_height_and_sit_inside_the_inset(cx: &mut TestAppContext) {
    let (view, cx) = open_list(
        cx,
        vec![
            session("default", SessionState::Running),
            session("work", SessionState::Stopped),
        ],
        development("default"),
    );
    let line = view.read_with(cx, |view, _| view.config.ui.line_height());
    let panel = bounds(cx, "menu-panel");
    let row = bounds(cx, "sessions-row-0");
    assert!(
        (row.size.height - px(line * 2. + 16.)).abs() <= px(0.5),
        "{row:?}"
    );
    assert!(row.left() >= panel.left() + px(7.), "{row:?} {panel:?}");
    let label = bounds(cx, "sessions-label-0");
    assert_eq!(label.left() - row.left(), px(10.));
    // The local header is a section heading: 8 px over and 4 px under its
    // line, where it used to pad 8 px all round.
    let header = bounds(cx, "sessions-local-header");
    assert!(header.size.height < px(line + 13.), "{header:?}");
}
