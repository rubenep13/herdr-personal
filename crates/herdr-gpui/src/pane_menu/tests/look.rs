use super::*;

/// Opens the pane menu on "inactive" in a window `width` wide, with the
/// longer right-click label.
fn open(width: f32, cx: &mut TestAppContext) -> (Entity<HerdrWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = crate::sidebar::layout_tests::fixture_window(window, cx);
        let mut snapshot = snapshot();
        snapshot.panes[1].right_click_passthrough = true;
        view.live.snapshot = Some(Arc::new(snapshot));
        view
    });
    cx.simulate_resize(size(px(width), px(600.)));
    cx.update(|window, cx| {
        view.update(cx, |v, cx| {
            v.open_pane_menu("inactive", point(px(20.), px(20.)), window, cx)
        });
        window.draw(cx).clear(cx);
    });
    (view, cx)
}

fn labels(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext) -> Vec<&'static str> {
    view.read_with(cx, |v, _| {
        let pane = v.menu.pane.as_ref().unwrap();
        pane.actions()
            .into_iter()
            .map(|action| action.label(&pane.target))
            .collect()
    })
}

#[gpui::test]
fn pane_rows_are_menu_rows_with_an_icon_each(cx: &mut TestAppContext) {
    let (view, cx) = open(800., cx);
    let count = labels(&view, cx).len();
    assert!(count > 0);
    for index in 0..count {
        let row = cx
            .debug_bounds(format!("pane-menu-{index}").leak())
            .unwrap();
        assert_eq!(row.size.height, px(32.), "row {index}");
        let icon = cx
            .debug_bounds(format!("pane-menu-{index}-icon").leak())
            .unwrap();
        assert_eq!(icon.size, size(px(16.), px(16.)), "row {index}");
    }
}

/// The icon never pushes a label out: the panel widens to the longest one,
/// and a narrow window ends it with an ellipsis inside the panel.
#[gpui::test]
fn long_pane_labels_stay_whole_beside_their_icon(cx: &mut TestAppContext) {
    let (view, cx) = open(800., cx);
    let labels = labels(&view, cx);
    assert!(labels.contains(&"Open This Menu on Right-Click"));
    let name = view.read_with(cx, |v, _| px(v.menu_look().scale().name));
    for (index, label) in labels.into_iter().enumerate() {
        let icon = cx
            .debug_bounds(format!("pane-menu-{index}-icon").leak())
            .unwrap();
        let text = cx
            .debug_bounds(format!("pane-menu-label-{index}").leak())
            .unwrap();
        let natural = cx.update(|window, _| {
            window
                .text_system()
                .shape_line(
                    label.into(),
                    name,
                    &[window.text_style().to_run(label.len())],
                    None,
                )
                .width
        });
        assert!(icon.right() <= text.left(), "{label:?}");
        assert!(text.size.width >= natural - px(0.5), "{label:?}");
    }

    let (_, cx) = open(360., cx);
    let panel = cx.debug_bounds("menu-panel").unwrap();
    assert!(
        panel.left() >= px(0.) && panel.right() <= px(360.),
        "{panel:?}"
    );
}
