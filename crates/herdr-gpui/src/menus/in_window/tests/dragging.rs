use super::*;

/// Observes presses reaching the chrome around the menu button. The
/// headless platform does not implement compositor moves; the button must
/// stop its presses before a title bar's move handler, while the chrome
/// around it still gets its own.
struct PressObserver {
    view: Entity<HerdrWindow>,
    presses: Rc<RefCell<Vec<MouseButton>>>,
}

impl Render for PressObserver {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let left = self.presses.clone();
        let right = self.presses.clone();
        let button = self
            .view
            .update(cx, |view, cx| view.application_menu_button(cx));
        div()
            .debug_selector(|| "menu-button-chrome".into())
            .size_full()
            .flex()
            .items_start()
            .p(px(20.))
            .on_mouse_down(MouseButton::Left, move |event, _, _| {
                left.borrow_mut().push(event.button)
            })
            .on_mouse_down(MouseButton::Right, move |event, _, _| {
                right.borrow_mut().push(event.button)
            })
            .child(button)
    }
}

#[gpui::test]
fn chrome_presses_reach_the_window_but_the_menu_button_keeps_its_own(cx: &mut TestAppContext) {
    let presses = Rc::new(RefCell::new(Vec::new()));
    let observed = presses.clone();
    let (root, cx) = cx.add_window_view(|window, cx| PressObserver {
        view: cx.new(|cx| {
            let view = fixture_window(window, cx);
            view.focus.focus(window, cx);
            view
        }),
        presses: observed,
    });
    let view = root.read_with(cx, |root, _| root.view.clone());
    cx.simulate_resize(size(px(400.), px(300.)));
    draw(cx);
    let chrome = cx.debug_bounds("menu-button-chrome").unwrap();
    cx.simulate_click(chrome.center(), Modifiers::none());
    assert_eq!(*presses.borrow(), [MouseButton::Left]);
    assert!(view.read_with(cx, |view, _| view.menu.page.is_none()));

    let button = cx.debug_bounds("app-menu-button").unwrap();
    cx.simulate_click(button.center(), Modifiers::none());
    draw(cx);
    assert_eq!(*presses.borrow(), [MouseButton::Left]);
    assert_eq!(
        view.read_with(cx, |view, _| view.menu.page),
        Some(Page::Application)
    );
    cx.simulate_event(MouseDownEvent {
        position: button.center(),
        button: MouseButton::Right,
        modifiers: Modifiers::none(),
        click_count: 1,
        first_mouse: false,
    });
    assert_eq!(*presses.borrow(), [MouseButton::Left]);
}
