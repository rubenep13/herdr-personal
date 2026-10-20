//! A setting's title keeps room to read at the smallest window and at a large
//! interface face: controls too wide to share the line move under it.
use super::*;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

/// The row's title is readable, its controls do not overlap it, and both
/// stay inside the row.
fn readable(cx: &mut VisualTestContext, row: &'static str) {
    let leak = |name: String| -> &'static str { Box::leak(name.into_boxed_str()) };
    let bounds = cx
        .debug_bounds(row)
        .unwrap_or_else(|| panic!("missing {row}"));
    let title = cx.debug_bounds(leak(format!("{row}-title"))).unwrap();
    let control = cx.debug_bounds(leak(format!("{row}-control"))).unwrap();
    assert!(
        title.size.width >= px(100.),
        "{row}: title squeezed to {title:?}"
    );
    assert!(
        title.right() <= control.left() || title.bottom() <= control.top(),
        "{row}: {title:?} overlaps {control:?}"
    );
    assert!(
        control.left() >= bounds.left() && control.right() <= bounds.right(),
        "{row}: {control:?} leaves {bounds:?}"
    );
}

fn page(view: &Entity<SettingsWindow>, section: Section, cx: &mut VisualTestContext) {
    view.update(cx, |view, cx| {
        view.section = section;
        cx.notify();
    });
    draw(cx);
}

#[gpui::test]
fn number_rows_stay_readable_in_the_smallest_window(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(680.), px(560.)));
    for (section, row) in [
        (Section::Sidebar, "sidebar-indent"),
        (Section::Fonts, "fonts-terminal"),
        (Section::Theme, "theme-padding-left"),
        (Section::Notifications, "notif-volume"),
        (Section::Tabs, "tabs-placement"),
    ] {
        page(&view, section, cx);
        // Long pages scroll; bring the row into the frame first.
        view.update(cx, |view, cx| {
            view.scroll_to_row(row, cx);
        });
        draw(cx);
        readable(cx, row);
    }
}

#[gpui::test]
fn font_rows_stay_readable_with_a_large_face(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    view.update(cx, |view, cx| {
        view.config.ui.size = 18.;
        cx.notify();
    });
    cx.simulate_resize(size(px(960.), px(780.)));
    page(&view, Section::Fonts, cx);
    readable(cx, "fonts-terminal");
}
