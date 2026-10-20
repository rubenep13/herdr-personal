//! A number with -/+ steppers, typed entry, and an optional drag slider.
use crate::design::SettingsLook;
use crate::search_input::SearchInput;
use gpui::{prelude::*, *};
use std::{cell::Cell, ops::RangeInclusive, rc::Rc, sync::Arc};

/// The value's cell and each stepper's width, at the default `[ui]` size.
const VALUE_WIDTH: f32 = 46.;
const STEPPER_WIDTH: f32 = 28.;

thread_local! {
    /// The last previewed value of the slider being dragged. Every Preview
    /// re-renders the page and rebuilds the slider's closures, so the value
    /// cannot live in them; GPUI runs one drag at a time, so one slot does.
    static PREVIEW: Cell<Option<(&'static str, f32)>> = const { Cell::new(None) };
}

fn remember_preview(id: &'static str, value: f32) {
    PREVIEW.with(|slot| slot.set(Some((id, value))));
}

/// The preview to commit on release; `None` when `id` is not being dragged,
/// so unrelated clicks commit nothing.
fn take_preview(id: &'static str) -> Option<f32> {
    PREVIEW.with(|slot| match slot.get() {
        Some((owner, value)) if owner == id => {
            slot.set(None);
            Some(value)
        }
        _ => None,
    })
}

pub(in crate::settings_window) struct Number {
    pub id: &'static str,
    pub value: f32,
    pub range: RangeInclusive<f32>,
    pub unit: &'static str,
    pub slider: bool,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::settings_window) enum NumberChange {
    Preview(f32),
    Commit(f32),
    /// A release that ends no drag of ours: the preview is dropped unwritten.
    Abandon,
}

/// The typed-entry state the view keeps while one field is being edited.
pub(in crate::settings_window) struct NumberEdit {
    pub field: crate::settings_window::inputs::NumberField,
    pub input: Entity<SearchInput>,
    pub invalid: bool,
    pub _blur: Subscription,
}

pub(in crate::settings_window) fn parse_number(
    text: &str,
    range: &RangeInclusive<f32>,
) -> Option<f32> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value = text.parse::<u16>().ok().map(f32::from)?;
    range.contains(&value).then_some(value)
}

pub(in crate::settings_window) fn slider_value(
    x: Pixels,
    bounds: Bounds<Pixels>,
    range: &RangeInclusive<f32>,
) -> f32 {
    let width = f32::from(bounds.size.width).max(1.);
    let fraction = ((f32::from(x) - f32::from(bounds.left())) / width).clamp(0., 1.);
    (range.start() + fraction * (range.end() - range.start())).round()
}

/// One step from `value`, rounded first so a fractional config value still
/// lands on a whole number; None for a non-finite value or step.
fn stepped_value(value: f32, delta: f32, range: &RangeInclusive<f32>) -> Option<f32> {
    if !value.is_finite() || !delta.is_finite() {
        return None;
    }
    Some((value.round() + delta.round()).clamp(*range.start(), *range.end()))
}

#[derive(Clone, Copy, PartialEq)]
struct SliderDrag(&'static str);

pub(in crate::settings_window) fn number(
    look: &SettingsLook,
    spec: Number,
    editing: Option<&NumberEdit>,
    on_change: impl Fn(NumberChange, &mut Window, &mut App) + 'static,
    on_edit: impl Fn(&mut Window, &mut App) + 'static,
) -> Div {
    let on_change = Rc::new(on_change);
    let id = spec.id;
    let enabled = spec.enabled;
    let range = spec.range.clone();
    let mut root = div()
        .debug_selector(move || id.into())
        .flex()
        .flex_wrap()
        .items_center()
        .gap(look.at(10.))
        .min_w_0()
        .when(!enabled, |root| root.opacity(0.5));
    if spec.slider {
        let fraction =
            ((spec.value - range.start()) / (range.end() - range.start()).max(1.)).clamp(0., 1.);
        let commit = {
            let on_change = on_change.clone();
            move |_: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                // A release with no drag running cannot be ours: a drag that
                // ended without reaching us (unmounted slider) left a stale
                // value behind.
                match (take_preview(id), cx.has_active_drag()) {
                    (Some(value), true) => on_change(NumberChange::Commit(value), window, cx),
                    (Some(_), false) => on_change(NumberChange::Abandon, window, cx),
                    (None, _) => {}
                }
            }
        };
        let track = div()
            .id((ElementId::from(id), "slider"))
            .debug_selector(move || format!("{id}-slider"))
            .w(look.at(150.))
            .h(look.at(16.))
            .flex()
            .items_center()
            .child(
                div()
                    .w_full()
                    .h(px(3.))
                    .rounded_full()
                    .bg(look.line())
                    .child(
                        div()
                            .h_full()
                            .w(relative(fraction))
                            .rounded_full()
                            .bg(look.accent()),
                    ),
            );
        root = root.child(if enabled {
            let on_change = on_change.clone();
            let range = range.clone();
            track
                .cursor_pointer()
                .on_drag(SliderDrag(id), move |_, _, _, cx| {
                    // A new drag starts from nothing, never a leftover value.
                    take_preview(id);
                    cx.new(|_| Empty)
                })
                .on_drag_move(move |event: &DragMoveEvent<SliderDrag>, window, cx| {
                    if event.drag(cx).0 == id {
                        let value = slider_value(event.event.position.x, event.bounds, &range);
                        remember_preview(id, value);
                        on_change(NumberChange::Preview(value), window, cx);
                    }
                })
                .on_mouse_up(MouseButton::Left, commit.clone())
                .on_mouse_up_out(MouseButton::Left, commit)
                .into_any_element()
        } else {
            track.into_any_element()
        });
    }
    let value_text = format!("{}", spec.value as i32);
    let border = match editing {
        Some(edit) if edit.invalid => look.danger(),
        _ => look.line(),
    };
    let value_width = look.at(VALUE_WIDTH);
    let shown = match editing {
        Some(edit) => div()
            .w(value_width)
            .child(edit.input.clone())
            .into_any_element(),
        None => div()
            .id((ElementId::from(id), "value"))
            .min_w(value_width)
            .h_full()
            .px(look.at(4.))
            .flex()
            .items_center()
            .justify_center()
            .gap(look.at(3.))
            .font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
            .text_color(look.title())
            .when(enabled, |value| value.cursor_text())
            .child(value_text)
            .child(div().text_color(look.muted()).child(spec.unit))
            .when(enabled, |value| {
                value.on_click(move |_, window, cx| on_edit(window, cx))
            })
            .into_any_element(),
    };
    let stepper_for = |which: &'static str, glyph: &'static str, delta: f32| {
        let on_change = on_change.clone();
        let range = range.clone();
        let value = spec.value;
        stepper(look, id, which, glyph, enabled, move |window, cx| {
            if let Some(next) = stepped_value(value, delta, &range) {
                on_change(NumberChange::Commit(next), window, cx)
            }
        })
    };
    // The value sits between its steppers, as the mockup draws it.
    let field = div()
        .debug_selector(move || format!("{id}-field"))
        .h(px(look.control_height()))
        .flex()
        .items_center()
        .rounded(look.at(8.))
        .border_1()
        .border_color(border)
        .bg(look.well())
        .child(stepper_for("minus", "\u{2212}", -1.))
        .child(shown)
        .child(stepper_for("plus", "+", 1.));
    #[cfg(all(feature = "integration-test", target_os = "macos"))]
    let field = field
        .relative()
        .child(crate::settings_window::controls::native_probe(format!(
            "{id}-field"
        )));
    root.child(field).child(
        div()
            .text_size(px(look.scale().detail))
            .text_color(look.muted())
            .child(format!(
                "{} to {}",
                *range.start() as i32,
                *range.end() as i32
            )),
    )
}

fn stepper(
    look: &SettingsLook,
    id: &'static str,
    which: &'static str,
    glyph: &'static str,
    enabled: bool,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let (fill, ink) = (look.selection(), look.title());
    div()
        .id((ElementId::from(id), which))
        .debug_selector(move || format!("{id}-{which}"))
        .w(look.at(STEPPER_WIDTH))
        .h_full()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(look.at(8.))
        .text_color(look.muted())
        .when(enabled, |stepper| {
            stepper
                .cursor_pointer()
                .hover(move |s| s.bg(fill).text_color(ink))
                .on_click(move |_, window, cx| on_click(window, cx))
        })
        .child(glyph)
}

#[cfg(test)]
mod tests;
