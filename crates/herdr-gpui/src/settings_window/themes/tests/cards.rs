//! The Theme page's cards and search in the redesigned look.
use super::super::appearance::source_label;
use super::*;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn theme_cards_have_a_forty_pixel_swatch(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(size(px(960.), px(1000.)));
    draw(cx);
    assert_eq!(
        cx.debug_bounds("settings-theme-thumbnail-0")
            .unwrap()
            .size
            .height,
        px(40.)
    );
    assert_eq!(
        cx.debug_bounds("settings-theme-row-0").unwrap().size.height,
        px(96.)
    );
}

#[gpui::test]
fn the_chosen_card_keeps_its_size(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(size(px(960.), px(1000.)));
    view.update(cx, |view, cx| view.select_settings_theme(0, cx));
    draw(cx);
    // Cards share the row's width up to a pixel of rounding.
    let near = |a: Size<Pixels>, b: Size<Pixels>| {
        a.height == b.height && (a.width - b.width).abs() <= px(1.)
    };
    let chosen = cx.debug_bounds("settings-theme-row-0").unwrap();
    let other = cx.debug_bounds("settings-theme-row-1").unwrap();
    assert!(near(chosen.size, other.size), "{chosen:?} {other:?}");
    let swatch = cx.debug_bounds("settings-theme-thumbnail-0").unwrap();
    let next = cx.debug_bounds("settings-theme-thumbnail-1").unwrap();
    assert!(
        near(swatch.size, next.size),
        "the ring takes no room from the swatch: {swatch:?} {next:?}"
    );
}

#[gpui::test]
fn the_theme_search_shows_its_count(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(size(px(960.), px(1000.)));
    draw(cx);
    let search = cx.debug_bounds("settings-theme-search").unwrap();
    let count = cx.debug_bounds("settings-theme-count").unwrap();
    assert!(
        count.left() > search.center().x && count.right() <= search.right(),
        "{count:?} at the right of {search:?}"
    );
    assert!(count.top() >= search.top() && count.bottom() <= search.bottom());
}

#[test]
fn the_omarchy_theme_names_its_source() {
    let app = |name: &str| Choice {
        scope: Scope::App,
        name: name.into(),
    };
    assert_eq!(source_label(&app(crate::config::OMARCHY_THEME)), "Omarchy");
    assert_eq!(source_label(&app("Nord")), "Built-in");
    assert_eq!(source_label(&app("/themes/mine")), "File");
    assert_eq!(source_label(&app("~/themes/mine")), "File");
    assert_eq!(source_label(&app("Catalog 001")), "Ghostty");
    let herdr = Choice {
        scope: Scope::Herdr,
        name: "catppuccin".into(),
    };
    assert_eq!(source_label(&herdr), "Herdr");
}
