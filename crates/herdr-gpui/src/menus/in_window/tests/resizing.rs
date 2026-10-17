use super::*;

struct InsetWindow {
    view: Entity<HerdrWindow>,
    inset: Pixels,
}

impl Render for InsetWindow {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p(self.inset).child(self.view.clone())
    }
}

#[gpui::test]
fn open_menu_follows_its_button_through_resizes_and_insets(cx: &mut TestAppContext) {
    let (root, cx) = cx.add_window_view(|window, cx| InsetWindow {
        view: cx.new(|cx| {
            let view = fixture_window(window, cx);
            view.focus.focus(window, cx);
            view
        }),
        inset: px(0.),
    });
    let view = root.read_with(cx, |root, _| root.view.clone());
    cx.simulate_resize(size(px(1200.), px(700.)));
    draw(cx);
    let button = cx.debug_bounds("app-menu-button").unwrap();
    cx.simulate_click(button.center(), Modifiers::none());
    draw(cx);

    for (width, inset) in [(360., 0.), (1200., 11.), (360., 11.), (1200., 0.)] {
        root.update(cx, |root, cx| {
            root.inset = px(inset);
            cx.notify();
        });
        cx.simulate_resize(size(px(width), px(700.)));
        for _ in 0..5 {
            draw(cx);
            let button = cx.debug_bounds("app-menu-button").unwrap();
            view.read_with(cx, |view, _| {
                assert_eq!(view.menu.page, Some(Page::Application));
                assert_eq!(view.menu.application_bar.bar.get(), button);
            });
            let panel = cx.debug_bounds("application-menu-panel").unwrap();
            assert!((panel.top() - button.bottom()).abs() <= px(1.));
            assert!(panel.left() >= px(inset) && panel.right() <= px(width - inset));
        }
    }
}
