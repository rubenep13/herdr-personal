use super::*;
use crate::config::Config;

/// Paints the builders the way a page does, at the default `[ui]` size.
struct LookHarness {
    /// The width the long row is squeezed into, so its description wraps.
    narrow: f32,
}

impl Render for LookHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let look = &SettingsLook::of(&Theme::default(), &Config::default().ui);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div().w(px(self.narrow)).child(
                    row(look, "look-row", "Selection marks")
                        .description(
                            "What selecting a workspace highlights beyond its own row, and how \
                             far that highlight reaches across the hosts below it.",
                        )
                        .control(toggle(look, "look-row-toggle", true, true)),
                ),
            )
            .child(
                group(look, "demo", "Demo", None)
                    .row(row(look, "look-a", "First").control(toggle(look, "a", true, true)))
                    .row(row(look, "look-b", "Second").control(toggle(look, "b", false, true)))
                    .row(row(look, "look-sub", "Dependent").sub()),
            )
            .child(segmented(
                look,
                "seg",
                &[(1u8, "One"), (2u8, "Two")],
                Some(1),
                true,
                |_, _, _| {},
            ))
            .child(dropdown(
                look,
                "look-dd",
                "Tree",
                false,
                true,
                None,
                |_, _| {},
                |_, _, _| {},
                |_, _| {},
            ))
            .child(number(
                look,
                Number {
                    id: "look-num",
                    value: 12.,
                    range: 0.0..=40.,
                    unit: "px",
                    slider: false,
                    enabled: true,
                },
                None,
                |_, _, _| {},
                |_, _| {},
            ))
    }
}

fn paint(cx: &mut TestAppContext) -> &mut VisualTestContext {
    let (_, cx) = cx.add_window_view(|_, _| LookHarness { narrow: 400. });
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx
}

#[gpui::test]
fn a_setting_row_is_at_least_forty_eight_tall(cx: &mut TestAppContext) {
    let cx = paint(cx);
    let short = cx.debug_bounds("look-a").unwrap();
    assert!(short.size.height >= px(48.), "{short:?}");
    let long = cx.debug_bounds("look-row").unwrap();
    assert!(
        long.size.height > px(48.),
        "a wrapped description grows the row: {long:?}"
    );
    let control = cx.debug_bounds("look-row-control").unwrap();
    let offset = (control.center().y - long.center().y).abs();
    assert!(offset <= px(1.), "control centred in the row: {offset:?}");
}

#[gpui::test]
fn a_group_puts_its_rows_in_one_card(cx: &mut TestAppContext) {
    let cx = paint(cx);
    let card = cx.debug_bounds("settings-card-demo").unwrap();
    let first = cx.debug_bounds("look-a").unwrap();
    let second = cx.debug_bounds("look-b").unwrap();
    assert!(
        card.contains(&first.origin)
            && card.contains(&(second.bottom_right() - point(px(1.), px(1.))))
    );
    assert!(
        cx.debug_bounds("settings-group-demo").is_some(),
        "the title stays"
    );
    let rule = cx.debug_bounds("settings-card-demo-rule-0").unwrap();
    assert!(rule.top() >= first.bottom() - px(0.5) && rule.bottom() <= second.top() + px(0.5));
    assert_eq!(
        rule.left() - card.left(),
        px(15.),
        "14 px past the card's 1 px border"
    );
    assert_eq!(card.right() - rule.right(), px(15.));
}

#[gpui::test]
fn a_dependent_row_steps_in(cx: &mut TestAppContext) {
    let cx = paint(cx);
    let card = cx.debug_bounds("settings-card-demo").unwrap();
    let title = cx.debug_bounds("look-sub-title").unwrap();
    assert!(
        title.left() - card.left() >= px(34.),
        "{title:?} in {card:?}"
    );
    assert!(cx.debug_bounds("look-sub-sub").is_some(), "the ↳ mark");
}

#[gpui::test]
fn segments_are_twenty_six_inside_a_thirty_box(cx: &mut TestAppContext) {
    let cx = paint(cx);
    assert_eq!(cx.debug_bounds("seg-0").unwrap().size.height, px(26.));
    assert_eq!(
        cx.debug_bounds("seg-segments").unwrap().size.height,
        px(30.)
    );
}

#[gpui::test]
fn the_dropdown_button_is_thirty_tall_and_at_least_170_wide(cx: &mut TestAppContext) {
    let cx = paint(cx);
    let button = cx.debug_bounds("look-dd").unwrap();
    assert_eq!(button.size.height, px(30.));
    assert!(button.size.width >= px(170.), "{button:?}");
}

#[gpui::test]
fn the_number_field_is_thirty_tall(cx: &mut TestAppContext) {
    let cx = paint(cx);
    assert_eq!(
        cx.debug_bounds("look-num-field").unwrap().size.height,
        px(30.)
    );
}
