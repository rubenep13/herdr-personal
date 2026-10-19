//! Shared row and control builders for the Settings pages. Pure rendering:
//! they take the page's [`SettingsLook`] and return elements; state and saves
//! stay on the view.
use crate::design::{ButtonKind, ButtonSize, SettingsLook, Tone};
use gpui::{prelude::*, *};
use std::rc::Rc;

mod number;
mod popover;
mod swatches;
pub(in crate::settings_window) use number::{
    Number, NumberChange, NumberEdit, number, parse_number,
};
pub(in crate::settings_window) use popover::{Dropdown, dropdown};
pub(in crate::settings_window) use swatches::{HostColorChange, parse_hex_input, swatch_row};

/// A setting's row at its least, at the default `[ui]` size; rows grow with
/// the face ([`SettingsLook::row_min_height`]).
pub(in crate::settings_window) const ROW_HEIGHT: f32 = 48.;
/// How far a dependent row steps in, and where its mark sits.
const SUB_INDENT: f32 = 34.;
const SUB_MARK: f32 = 18.;

/// Where a setting is stored: shared with every machine or local to this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::settings_window) enum Source {
    Shared,
    Local,
}

/// The chip naming where a setting is kept, in the theme's colours.
pub(in crate::settings_window) fn tag(look: &SettingsLook, source: Source) -> Div {
    let (label, tone) = match source {
        Source::Shared => ("Shared", Tone::Shared),
        Source::Local => ("Local", Tone::Local),
    };
    look.chip(format!("tag-{}", label.to_lowercase()), label, tone)
}

/// A section's title: sentence case, with the chip of the place its rows are
/// kept when they all share one.
pub(in crate::settings_window) fn section(
    look: &SettingsLook,
    title: &'static str,
    tag_source: Option<Source>,
) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(look.at(8.))
        .mt(look.at(24.))
        .mb(look.at(8.))
        .mx(look.at(2.))
        .text_size(px(look.scale().secondary))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(look.text())
        .child(title)
        .children(tag_source.map(|source| tag(look, source)))
}

/// A section: its title, an optional lead under it, and its rows together
/// in one card, a rule between each two.
pub(in crate::settings_window) struct Group {
    look: SettingsLook,
    selector: &'static str,
    title: &'static str,
    tag: Option<Source>,
    lead: Option<SharedString>,
    children: Vec<AnyElement>,
}

pub(in crate::settings_window) fn group(
    look: &SettingsLook,
    selector: &'static str,
    title: &'static str,
    tag: Option<Source>,
) -> Group {
    Group {
        look: *look,
        selector,
        title,
        tag,
        lead: None,
        children: Vec::new(),
    }
}

impl Group {
    /// Text under the title, outside the card, that explains the section.
    pub(in crate::settings_window) fn lead(mut self, text: impl Into<SharedString>) -> Self {
        self.lead = Some(text.into());
        self
    }

    pub(in crate::settings_window) fn row(mut self, row: impl IntoElement) -> Self {
        self.children.push(row.into_any_element());
        self
    }

    pub(in crate::settings_window) fn rows(
        mut self,
        rows: impl IntoIterator<Item = impl IntoElement>,
    ) -> Self {
        self.children
            .extend(rows.into_iter().map(IntoElement::into_any_element));
        self
    }

    /// Content of the card that is not a setting's row, such as a grid or a
    /// form, padded like a row.
    pub(in crate::settings_window) fn body(mut self, element: impl IntoElement) -> Self {
        let padding = self.look.at(14.);
        self.children
            .push(div().p(padding).child(element).into_any_element());
        self
    }
}

impl IntoElement for Group {
    type Element = Div;
    fn into_element(self) -> Div {
        let Self {
            look,
            selector,
            title,
            tag,
            lead,
            children,
        } = self;
        let mut card = look.card(format!("settings-card-{selector}"));
        for (index, child) in children.into_iter().enumerate() {
            if index > 0 {
                let rule = index - 1;
                card = card.child(
                    look.rule()
                        .debug_selector(move || format!("settings-card-{selector}-rule-{rule}")),
                );
            }
            card = card.child(child);
        }
        div()
            .flex()
            .flex_col()
            .child(
                section(&look, title, tag)
                    .debug_selector(move || format!("settings-group-{selector}")),
            )
            .children(lead.map(|lead| {
                div()
                    .mx(look.at(2.))
                    .mb(look.at(8.))
                    .text_size(px(look.scale().secondary))
                    .line_height(relative(1.5))
                    .text_color(look.muted())
                    .child(lead)
            }))
            .child(card)
    }
}

pub(in crate::settings_window) struct Row {
    look: SettingsLook,
    id: SharedString,
    title: SharedString,
    description: Option<SharedString>,
    tag: Option<Source>,
    disabled: Option<SharedString>,
    sub: bool,
    controls: Vec<AnyElement>,
}

pub(in crate::settings_window) fn row(
    look: &SettingsLook,
    id: impl Into<SharedString>,
    title: impl Into<SharedString>,
) -> Row {
    Row {
        look: *look,
        id: id.into(),
        title: title.into(),
        description: None,
        tag: None,
        disabled: None,
        sub: false,
        controls: Vec::new(),
    }
}

impl Row {
    pub(in crate::settings_window) fn description(mut self, text: impl Into<SharedString>) -> Self {
        self.description = Some(text.into());
        self
    }
    pub(in crate::settings_window) fn tag(mut self, source: Source) -> Self {
        self.tag = Some(source);
        self
    }
    /// Dims the row's text and shows `reason` in place of the description.
    /// Its controls dim themselves when the caller builds them disabled.
    pub(in crate::settings_window) fn disabled(mut self, reason: impl Into<SharedString>) -> Self {
        self.disabled = Some(reason.into());
        self
    }
    /// A row that depends on the one above it: stepped in, with a ↳.
    pub(in crate::settings_window) fn sub(mut self) -> Self {
        self.sub = true;
        self
    }
    pub(in crate::settings_window) fn control(mut self, control: impl IntoElement) -> Self {
        self.controls.push(control.into_any_element());
        self
    }
}

impl IntoElement for Row {
    type Element = Div;
    fn into_element(self) -> Div {
        let look = self.look;
        let scale = look.scale();
        let id = self.id.clone();
        let title_id = format!("{id}-title");
        let desc_id = format!("{id}-description");
        let tag_id = format!("{id}-tag");
        let sub_id = format!("{id}-sub");
        let control_id = format!("{id}-control");
        let dimmed = self.disabled.is_some();
        let tracked = id.clone();
        let reason_id = format!("{id}-reason");
        // A disabled row shows its reason instead of the description.
        let (description, desc_id) = match self.disabled {
            Some(reason) => (Some(reason), reason_id),
            None => (self.description, desc_id),
        };
        div()
            .debug_selector(move || id.to_string())
            .relative()
            .flex()
            .items_center()
            .gap(look.at(16.))
            .min_h(px(look.row_min_height()))
            .py(look.at(9.))
            .px(look.at(14.))
            .when(self.sub, |row| row.pl(look.at(SUB_INDENT)))
            .child(super::search::track_bounds(tracked))
            .child(
                // Only the text dims: a control a disabled row cannot use is
                // built disabled and dims itself, and GPUI multiplies nested
                // opacity, so dimming the whole row would show it at 25 %.
                div()
                    .flex_1()
                    .min_w_0()
                    .when(dimmed, |text| text.opacity(0.5))
                    .child(
                        div()
                            .debug_selector(move || title_id.clone())
                            .relative()
                            .flex()
                            .items_center()
                            .gap(look.at(7.))
                            .text_size(px(scale.name))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(look.title())
                            .when(self.sub, |title| {
                                title.child(
                                    div()
                                        .debug_selector(move || sub_id.clone())
                                        .absolute()
                                        .left(-look.at(SUB_MARK))
                                        .font_weight(FontWeight::NORMAL)
                                        .text_color(look.muted())
                                        .child("↳"),
                                )
                            })
                            .child(self.title)
                            .children(self.tag.map(|source| {
                                tag(&look, source).debug_selector(move || tag_id.clone())
                            })),
                    )
                    .children(description.map(|text| {
                        div()
                            .debug_selector(move || desc_id.clone())
                            .mt(look.at(2.))
                            .text_size(px(scale.secondary))
                            .line_height(relative(1.45))
                            .text_color(look.muted())
                            .child(text)
                    })),
            )
            .child(
                div()
                    .debug_selector(move || control_id.clone())
                    .flex()
                    .items_center()
                    .gap(look.at(8.))
                    .flex_none()
                    .children(self.controls),
            )
    }
}

/// The switch track alone; the row carries the label.
pub(in crate::settings_window) fn toggle(
    look: &SettingsLook,
    id: impl Into<SharedString>,
    checked: bool,
    enabled: bool,
) -> Stateful<Div> {
    let id = id.into();
    look.switch(checked)
        .id(ElementId::Name(id.clone()))
        .debug_selector(move || id.to_string())
        .when(enabled, |track| track.cursor_pointer())
        .when(!enabled, |track| track.opacity(0.5))
}

/// A plain button; the caller adds `.on_click` when it is enabled.
pub(in crate::settings_window) fn button(
    look: &SettingsLook,
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    enabled: bool,
) -> Stateful<Div> {
    let id = id.into();
    look.button(
        ElementId::Name(id.clone()),
        id.to_string(),
        label,
        ButtonKind::Normal,
        ButtonSize::Regular,
        enabled,
    )
}

pub(in crate::settings_window) fn segmented<T: Copy + PartialEq + 'static>(
    look: &SettingsLook,
    id: &'static str,
    options: &[(T, &'static str)],
    selected: Option<T>,
    enabled: bool,
    on_pick: impl Fn(T, &mut Window, &mut App) + 'static,
) -> Div {
    let options = options
        .iter()
        .map(|(value, label)| (*value, (*label).into_any_element()))
        .collect();
    segmented_elements(look, id, options, selected, enabled, |_| true, on_pick)
}

/// Like [`segmented`], with an element per segment (e.g. coloured status marks).
/// `live` says which segments respond while the strip is enabled; the others
/// are dimmed and ignore clicks.
pub(in crate::settings_window) fn segmented_elements<T: Copy + PartialEq + 'static>(
    look: &SettingsLook,
    id: &'static str,
    options: Vec<(T, AnyElement)>,
    selected: Option<T>,
    enabled: bool,
    live: impl Fn(T) -> bool,
    on_pick: impl Fn(T, &mut Window, &mut App) + 'static,
) -> Div {
    let on_pick = Rc::new(on_pick);
    let hover = look.text();
    div()
        .debug_selector(move || format!("{id}-segments"))
        .flex()
        .flex_none()
        .items_center()
        // The 2 px around the segments include the box's 1 px ring.
        .p(look.at(2.) - px(1.))
        .rounded(look.at(8.))
        .border_1()
        .border_color(look.line())
        .bg(look.well())
        .when(!enabled, |strip| strip.opacity(0.5))
        .children(
            options
                .into_iter()
                .enumerate()
                .map(|(index, (value, label))| {
                    let on = selected == Some(value);
                    let segment_live = enabled && live(value);
                    let on_pick = on_pick.clone();
                    div()
                        .id((id, index))
                        .debug_selector(move || format!("{id}-{index}"))
                        .h(px(look.segment_height()))
                        .px(look.at(10.))
                        .flex()
                        .items_center()
                        .gap(look.at(6.))
                        .rounded(look.at(6.))
                        .whitespace_nowrap()
                        .text_size(look.at(12.5))
                        .text_color(if on { look.title() } else { look.muted() })
                        .when(on, |seg| {
                            seg.bg(look.selection()).font_weight(FontWeight::MEDIUM)
                        })
                        .when(enabled && !segment_live, |seg| seg.opacity(0.5))
                        .when(segment_live, |seg| seg.cursor_pointer())
                        .when(segment_live && !on, move |seg| {
                            seg.hover(move |seg| seg.text_color(hover))
                                .on_click(move |_, window, cx| on_pick(value, window, cx))
                        })
                        .child(label)
                }),
        )
}

#[cfg(test)]
mod tests;
