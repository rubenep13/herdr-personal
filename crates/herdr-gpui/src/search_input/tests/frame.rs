//! A search input keeps its own frame wherever it is not asked for another:
//! the dialogs' fields ask for Settings' frame, and the rest of the app
//! (browser toolbar, find bar, Settings' number editor) keeps today's.

use super::*;
use gpui::{AppContext, Entity, px, size};

/// A column holding the input, as the app's bars and forms hold one.
struct Holder(Entity<SearchInput>);

impl Render for Holder {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_start()
            .w(px(300.))
            .child(self.0.clone())
    }
}

fn height(frame: Option<Frame>, cx: &mut gpui::TestAppContext) -> Pixels {
    let (holder, cx) = cx.add_window_view(|_, cx| Holder(cx.new(SearchInput::new)));
    cx.simulate_resize(size(px(400.), px(200.)));
    cx.update(|window, cx| {
        let input = holder.read(cx).0.clone();
        input.update(cx, |input, cx| {
            if let Some(frame) = frame {
                input.set_frame(frame, cx);
            }
        });
        window.draw(cx).clear(cx);
    });
    cx.debug_bounds("theme-search").unwrap().size.height
}

#[gpui::test]
fn an_input_keeps_its_own_frame_unless_asked(cx: &mut gpui::TestAppContext) {
    let line = Config::default().ui.line_height();
    // Its own frame: a line of text, 4 px of padding above and below, and
    // the hairline.
    let own = height(None, cx);
    // Layout rounds to device pixels.
    assert!((own - px(line + 10.)).abs() <= px(0.5), "{own:?}");
}

#[gpui::test]
fn a_dialog_field_wears_settings_frame(cx: &mut gpui::TestAppContext) {
    assert_eq!(height(Some(Frame::Field), cx), px(30.));
}
