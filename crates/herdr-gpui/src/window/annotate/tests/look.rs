use super::*;

/// The note to an agent is the menus' panel, its field in Settings' frame
/// and its footer ending in Send, the primary action.
#[gpui::test]
fn a_terminal_note_ends_in_its_buttons(cx: &mut TestAppContext) {
    let (view, cx) = window(cx, shown(Some("working"), false));
    select(&view, cx);
    write(&view, cx, "Check the database first.");
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    let queue = cx.debug_bounds("terminal-note-queue").unwrap();
    let send = cx.debug_bounds("terminal-note-send").unwrap();
    assert!(send.left() > queue.left());
    assert_eq!(queue.size.height, px(26.));
    assert_eq!(send.size.height, px(26.));
    let card = cx.debug_bounds("terminal-note").unwrap();
    assert!(card.contains(&send.origin) && card.contains(&send.bottom_right()));
    assert_eq!(
        cx.debug_bounds("terminal-note-input").unwrap().size.height,
        px(30.)
    );
}

/// At large type the note grows with its pieces, so Queue and Send stay in
/// the card rather than below its fold.
#[gpui::test]
fn a_large_terminal_note_keeps_its_buttons_in_view(cx: &mut TestAppContext) {
    let (view, cx) = window(cx, shown(Some("working"), false));
    view.update(cx, |view, cx| {
        view.config.ui.size = 18.;
        cx.notify();
    });
    select(&view, cx);
    write(&view, cx, "Check the database first.");
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    let card = cx.debug_bounds("terminal-note").unwrap();
    for selector in ["terminal-note-queue", "terminal-note-send"] {
        let button = cx.debug_bounds(selector).unwrap();
        assert!(
            button.bottom() <= card.bottom() && button.right() <= card.right(),
            "{selector}: {button:?} in {card:?}"
        );
    }
}
