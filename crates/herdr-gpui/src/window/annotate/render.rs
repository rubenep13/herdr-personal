use super::*;
use crate::{
    design::{ButtonKind, Header},
    fonts::StyledFont,
};

/// The note's size at `[ui] size = 12`: heading 52, padding 12 above and
/// below, field 30, gap 10 and buttons 26; and its three hairlines.
const NOTE_WIDTH: f32 = 410.;
const NOTE_HEIGHT: f32 = 52. + 2. * 12. + 30. + 10. + 26.;
const NOTE_LINES: f32 = 3.;

/// Prefer just below the selection, flip above it at the bottom edge, and
/// clamp to the pane. Both the pill and composer use these same coordinates.
fn placement(
    pane: Bounds<Pixels>,
    anchor: Bounds<Pixels>,
    desired: Size<Pixels>,
) -> Bounds<Pixels> {
    let inset = px(8.).min(pane.size.width / 2.).min(pane.size.height / 2.);
    let width = desired
        .width
        .min((pane.size.width - inset * 2.).max(px(0.)));
    let height = desired
        .height
        .min((pane.size.height - inset * 2.).max(px(0.)));
    let left = (anchor.right() - width).clamp(pane.left() + inset, pane.right() - inset - width);
    let below = anchor.bottom() + px(6.);
    let top = if below + height <= pane.bottom() - inset {
        below
    } else {
        anchor.top() - px(6.) - height
    }
    .clamp(pane.top() + inset, pane.bottom() - inset - height);
    Bounds::new(point(left, top), size(width, height))
}

impl HerdrWindow {
    /// The only pointer-occluding area is the pill or the compact composer.
    /// The terminal keeps focus until the user explicitly chooses Annotate.
    pub(crate) fn render_terminal_note(
        &self,
        surface: Option<&herdr_client::protocol::PaneSurfaceFrame>,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        if self.menu.page.is_some() {
            return None;
        }
        let surface = surface.filter(|surface| surface.popup.is_none())?;
        let composer = self.terminal_notes.composer.as_ref();
        let selection = match composer {
            Some(composer) => &composer.selection,
            None => {
                &self
                    .terminal_notes
                    .offer
                    .as_ref()
                    .filter(|offer| offer.current(self))?
                    .selection
            }
        };
        let pane_id = selection.pane_id()?;
        let pane = surface.panes.iter().find(|pane| pane.pane_id == pane_id)?;
        let cell_height = self.config.terminal.line_height();
        let pane_bounds = Bounds::new(
            point(
                px(f32::from(pane.rect.x) * self.cell_width),
                px(f32::from(pane.rect.y) * cell_height),
            ),
            size(
                px(f32::from(pane.rect.width) * self.cell_width),
                px(f32::from(pane.rect.height) * cell_height),
            ),
        );
        let anchor = selection
            .rows(surface, self.cell_width, cell_height)
            .last()
            .map(|(row, columns)| {
                Bounds::from_corners(
                    point(
                        px(f32::from(columns.start) * self.cell_width),
                        px(f32::from(row) * cell_height),
                    ),
                    point(
                        px(f32::from(columns.end) * self.cell_width),
                        px((f32::from(row) + 1.) * cell_height),
                    ),
                )
            })
            .unwrap_or(Bounds::new(
                pane_bounds.origin,
                size(px(0.), px(cell_height)),
            ));
        let mut font = self.config.ui.clone();
        font.size = font.size.max(14.);
        let desired = if composer.is_some() {
            // The pieces it is drawn from, which grow with `[ui]`: the
            // dialog heading, the body's padding, the field, the gap and the
            // buttons' row, and the panel's and heading's hairlines.
            let look = self.view_look();
            size(look.at(NOTE_WIDTH), look.at(NOTE_HEIGHT) + px(NOTE_LINES))
        } else {
            size(px(font.size * 6. + 48.), px(font.line_height() + 12.))
        };
        let bounds = placement(pane_bounds, anchor, desired);
        let theme = &self.theme;
        let look = self.view_look();
        let hover = look.hover();
        let content = match composer {
            Some(composer) => self.terminal_note_card(composer, cx),
            // The menus' panel in small, as the find bar's.
            None => look
                .menu()
                .panel(div())
                .id("terminal-annotate")
                .debug_selector(|| "terminal-annotate".into())
                .occlude()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .gap(look.at(8.))
                .px(look.at(12.))
                .rounded(look.at(crate::window::find::SMALL_PANEL_RADIUS))
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
                .on_mouse_down_out(
                    cx.listener(|this, _, _, cx| this.clear_terminal_annotation_offer(cx)),
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_terminal_annotation_offer(window, cx)
                }))
                .child(
                    svg()
                        .path("icons/note.svg")
                        .flex_none()
                        .size(px(16.))
                        .text_color(look.muted()),
                )
                .child(div().min_w_0().truncate().child("Annotate")),
        };
        Some(
            div()
                .absolute()
                .left(bounds.left())
                .top(bounds.top())
                .w(bounds.size.width)
                .h(bounds.size.height)
                .text_font(&font)
                .text_size(px(font.size))
                .line_height(px(font.line_height()))
                .text_color(rgb(theme.foreground))
                .child(content),
        )
    }

    fn terminal_note_card(&self, composer: &Composer, cx: &mut Context<Self>) -> Stateful<Div> {
        let look = self.view_look();
        let dialog = look.dialog();
        let queued = self.terminal_notes.queued.len();
        let hint = if queued > 0 {
            format!("{queued} queued · Enter to send")
        } else if composer.target.is_none() {
            "Copies to clipboard".into()
        } else {
            "Enter to send".into()
        };
        let close = look
            .menu()
            .close_button("terminal-note-cancel")
            .on_click(cx.listener(|this, _, window, cx| this.close_terminal_note(window, cx)));
        look.menu()
            .panel(div())
            .id("terminal-note")
            .debug_selector(|| "terminal-note".into())
            .occlude()
            .size_full()
            .overflow_y_scroll()
            .on_key_down(cx.listener(Self::terminal_note_key_down))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(dialog.header(
                Header {
                    selector: "terminal-note-header",
                    icon: "icons/note.svg",
                    title: ("terminal-note-title", "Note to agent"),
                    sub: None,
                    danger: false,
                },
                close,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(look.at(10.))
                    .p(look.at(12.))
                    .child(
                        look.settings()
                            .field("terminal-note-input".into(), true)
                            .min_w_0()
                            .child(div().flex_1().min_w_0().child(composer.input.clone())),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(look.at(6.))
                            .child(
                                div()
                                    .debug_selector(|| "terminal-note-hint".into())
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(look.scale().detail))
                                    .text_color(look.muted())
                                    .child(hint),
                            )
                            .child(
                                look.button(
                                    "terminal-note-queue",
                                    "terminal-note-queue".into(),
                                    None,
                                    "Queue",
                                    ButtonKind::Normal,
                                    true,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| this.add_terminal_note(false, window, cx),
                                )),
                            )
                            .child(
                                look.button(
                                    "terminal-note-send",
                                    "terminal-note-send".into(),
                                    None,
                                    "Send",
                                    ButtonKind::Primary,
                                    true,
                                )
                                .on_click(cx.listener(
                                    |this, _, window, cx| this.add_terminal_note(true, window, cx),
                                )),
                            ),
                    ),
            )
    }
}
