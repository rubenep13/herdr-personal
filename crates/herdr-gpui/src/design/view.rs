//! The look the views that are not terminals share since the redesign's
//! fifth phase: Devices, the orchestrator, Review, the browser, Code, the
//! Git graph, Logs and the usage panel. It composes the menus', Settings'
//! and dialogs' looks and adds what only a view has: its bar, icon buttons,
//! pills and a small dropdown, column headings and inset rows, side panels
//! and an empty state. Behaviour stays with each view; these pieces only
//! paint, so no view draws a bar or a row of its own again.

use super::{
    ButtonKind, ButtonSize, DialogLook, ICON_BUTTON, ICON_BUTTON_RADIUS, ItemState, Leading,
    MenuLook, SELECTION_RADIUS, SettingsLook, Tone, TypeScale,
};
use crate::config::{FontConfig, Theme, mix};
use gpui::{
    Div, ElementId, FontWeight, Pixels, Rgba, SharedString, Stateful, div, prelude::*, px, rgb, svg,
};

/// The `[ui]` size the measures below are given at; they grow with the face.
const BASE_SIZE: f32 = 12.;
/// A view's bar, as tall as the tab subheader.
const BAR: f32 = 40.;
const BAR_PADDING_LEFT: f32 = 14.;
const BAR_PADDING_RIGHT: f32 = 10.;
const BAR_GAP: f32 = 6.;
const BAR_ICON: f32 = 15.;
/// Between the view's icon, its title and its context.
const TITLE_GAP: f32 = 8.;
const CONTEXT_GAP: f32 = 2.;
/// How readily a bar's title gives way next to the fields and the context
/// beside it: barely, so they narrow first and the title only once they
/// cannot.
const TITLE_SHRINK: f32 = 0.01;
/// The thin upright line between groups of a bar's controls.
const BAR_RULE: f32 = 18.;
const BAR_RULE_MARGIN: f32 = 4.;
const ICON: f32 = 15.;
/// A bar's controls: a field, a segmented control, a pill, a dropdown.
const FIELD: f32 = 28.;
const CONTROL: f32 = 26.;
const CONTROL_RADIUS: f32 = 8.;
const CONTROL_PADDING: f32 = 10.;
const CONTROL_GAP: f32 = 6.;
/// One segment inside its box, and the box's inner padding.
const SEGMENT_PADDING: f32 = 9.;
const SEGMENT_RADIUS: f32 = 6.;
const SEGMENT_INSET: f32 = 2.;
const PILL_MARK: f32 = 12.;
/// A small button's icon, before its label.
const BUTTON_ICON: f32 = 14.;
const BUTTON_ICON_GAP: f32 = 6.;
/// A view's own menu: its padding, and its least width.
const MENU_PADDING: f32 = 6.;
const MENU_CHECK: f32 = 16.;
const DROPDOWN_PADDING_RIGHT: f32 = 8.;
const DROPDOWN_ICON: f32 = 13.;
/// The row of a list's search and filters.
const FILTERS: f32 = 46.;
const FILTERS_PADDING: f32 = 12.;
/// A list's column headings: the list's inset and a row's padding.
const COLUMNS: f32 = 28.;
const COLUMNS_PADDING: f32 = 18.;
/// A list keeps its rows this far from its sides.
const LIST_INSET: f32 = 8.;
const ROW: f32 = 32.;
const TWO_LINE_ROW: f32 = 44.;
const ROW_PADDING: f32 = 10.;
const ROW_GAP: f32 = 10.;
const SIDE_HEADER: f32 = 40.;
const SIDE_PADDING_LEFT: f32 = 14.;
const SIDE_PADDING_RIGHT: f32 = 8.;
const CARD_HEADER: f32 = 44.;
const CARD_PADDING_RIGHT: f32 = 10.;
const HEADER_GAP: f32 = 8.;
/// The empty state's tile, its corners and its icon.
const EMPTY_TILE: f32 = 40.;
const EMPTY_RADIUS: f32 = 10.;
const EMPTY_ICON: f32 = 20.;
const EMPTY_GAP: f32 = 12.;
const EMPTY_PADDING: f32 = 20.;
const STATUS_DOT: f32 = 7.;
const STATUS_RING: f32 = 1.5;
/// A hovered row, as the Settings page's rows.
const HOVER_PERCENT: u32 = 60;
/// A lit pill's wash of the accent.
const PILL_PERCENT: u32 = 15;
/// The empty state's tile's wash of the accent.
const TILE_PERCENT: u32 = 14;

/// Where a side panel sits, so its hairline faces the content beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edge {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ViewLook {
    dialog: DialogLook,
    background: Rgba,
    line: Rgba,
    /// A side panel's fill, a step darker than the view, as a dialog's well.
    side: Rgba,
    hover: Rgba,
    selection: Rgba,
    title: Rgba,
    text: Rgba,
    muted: Rgba,
    accent: Rgba,
    /// The accent as text on its own wash, lifted where the theme's own
    /// would not read there.
    accent_ink: Rgba,
    pill: Rgba,
    tile: Rgba,
    scale: TypeScale,
    /// How far the face is from the size the measures are given at.
    zoom: f32,
}

impl ViewLook {
    pub(crate) fn of(theme: &Theme, font: &FontConfig, light: bool) -> Self {
        Self {
            dialog: DialogLook::of(theme, font, light),
            background: rgb(theme.background),
            line: rgb(theme.active),
            side: rgb(theme.surface),
            hover: rgb(mix(theme.background, theme.active, HOVER_PERCENT)),
            selection: rgb(theme.active),
            title: rgb(theme.foreground),
            text: rgb(theme.subtext()),
            muted: rgb(theme.muted),
            accent: rgb(theme.primary()),
            accent_ink: rgb(theme.ink(theme.primary())),
            pill: rgb(mix(theme.background, theme.primary(), PILL_PERCENT)),
            tile: rgb(mix(theme.background, theme.primary(), TILE_PERCENT)),
            scale: TypeScale::of(font),
            zoom: font.size / BASE_SIZE,
        }
    }

    pub(crate) fn menu(&self) -> MenuLook {
        self.dialog.menu()
    }

    pub(crate) fn settings(&self) -> SettingsLook {
        self.dialog.settings()
    }

    pub(crate) fn dialog(&self) -> DialogLook {
        self.dialog
    }

    /// `measure`, given at `[ui] size = 12`, at this face's size.
    pub(crate) fn at(&self, measure: f32) -> Pixels {
        px(measure * self.zoom)
    }

    pub(crate) fn scale(&self) -> TypeScale {
        self.scale
    }

    pub(crate) fn row_height(&self, two_lines: bool) -> f32 {
        (if two_lines { TWO_LINE_ROW } else { ROW }) * self.zoom
    }

    /// The view's fill, and its bar's.
    pub(crate) fn background(&self) -> Rgba {
        self.background
    }

    pub(crate) fn line(&self) -> Rgba {
        self.line
    }

    pub(crate) fn side(&self) -> Rgba {
        self.side
    }

    pub(crate) fn hover(&self) -> Rgba {
        self.hover
    }

    pub(crate) fn selection(&self) -> Rgba {
        self.selection
    }

    pub(crate) fn title(&self) -> Rgba {
        self.title
    }

    pub(crate) fn text(&self) -> Rgba {
        self.text
    }

    pub(crate) fn muted(&self) -> Rgba {
        self.muted
    }

    pub(crate) fn accent(&self) -> Rgba {
        self.accent
    }

    /// A lit pill's text and fill.
    pub(crate) fn pill_on(&self) -> (Rgba, Rgba) {
        (self.accent_ink, self.pill)
    }

    /// The bar along a view's top. The caller adds its title and controls;
    /// what does not fit is clipped rather than growing the bar.
    pub(crate) fn bar(&self, selector: impl Into<String>) -> Div {
        let selector = selector.into();
        div()
            .debug_selector(move || selector)
            .flex_none()
            .w_full()
            .min_w_0()
            .h(self.at(BAR))
            .pl(self.at(BAR_PADDING_LEFT))
            .pr(self.at(BAR_PADDING_RIGHT))
            .flex()
            .items_center()
            .gap(self.at(BAR_GAP))
            .overflow_hidden()
            .whitespace_nowrap()
            .bg(self.background)
            .border_b_1()
            .border_color(self.line)
            .text_size(px(self.scale.name))
            .text_color(self.text)
    }

    /// A bar's icon, title and context. The title keeps its width; the
    /// context gives way first and ends in an ellipsis.
    pub(crate) fn bar_title(
        &self,
        icon: Option<&'static str>,
        title: impl Into<SharedString>,
        context: Option<SharedString>,
    ) -> Div {
        div()
            .min_w_0()
            .flex_shrink(TITLE_SHRINK)
            .overflow_hidden()
            .flex()
            .items_center()
            .gap(self.at(TITLE_GAP))
            .when_some(icon, |row, icon| {
                row.child(
                    svg()
                        .path(icon)
                        .size(self.at(BAR_ICON))
                        .flex_none()
                        .text_color(self.muted),
                )
            })
            .child(
                div()
                    .min_w_0()
                    .flex_shrink(TITLE_SHRINK)
                    .truncate()
                    .text_size(px(self.scale.name))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.title)
                    .child(title.into()),
            )
            .when_some(context, |row, context| {
                row.child(
                    div()
                        .min_w_0()
                        .ml(self.at(CONTEXT_GAP))
                        .truncate()
                        .text_size(px(self.scale.secondary))
                        .text_color(self.muted)
                        .child(context),
                )
            })
    }

    /// The thin upright line between groups of a bar's controls.
    pub(crate) fn bar_rule(&self) -> Div {
        div()
            .flex_none()
            .w(px(1.))
            .h(self.at(BAR_RULE))
            .mx(self.at(BAR_RULE_MARGIN))
            .bg(self.line)
    }

    /// A square button holding one icon, as the header's: filled while
    /// hovered, and while on when it toggles something. A disabled one fades
    /// and takes no pointer. The caller adds its click.
    pub(crate) fn icon_button(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        icon: &'static str,
        on: bool,
        enabled: bool,
    ) -> Stateful<Div> {
        let hover = self.selection;
        div()
            .id(id)
            .debug_selector(move || selector)
            .flex_none()
            .size(self.at(ICON_BUTTON))
            .flex()
            .items_center()
            .justify_center()
            .rounded(self.at(ICON_BUTTON_RADIUS))
            .when(on, |button| button.bg(self.selection))
            .when(enabled, |button| {
                button
                    .cursor_pointer()
                    .hover(move |button| button.bg(hover))
            })
            .when(!enabled, |button| button.opacity(0.4))
            .child(
                svg()
                    .path(icon)
                    .size(self.at(ICON))
                    .flex_none()
                    .text_color(if on { self.title } else { self.muted }),
            )
    }

    /// A small button of a bar, a panel or a card, its icon before its
    /// label in the label's colour. The caller adds its click.
    pub(crate) fn button(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        icon: Option<&'static str>,
        label: impl Into<SharedString>,
        kind: ButtonKind,
        enabled: bool,
    ) -> Stateful<Div> {
        let settings = self.settings();
        let (_, _, ink) = settings.button_colours(kind);
        settings.button(
            id,
            selector,
            div()
                .flex()
                .items_center()
                .gap(self.at(BUTTON_ICON_GAP))
                .when_some(icon, |label, icon| {
                    label.child(
                        svg()
                            .path(icon)
                            .size(self.at(BUTTON_ICON))
                            .flex_none()
                            .text_color(ink),
                    )
                })
                .child(label.into()),
            kind,
            ButtonSize::Small,
            enabled,
        )
    }

    /// A view's own menu, in the menus' panel. The caller adds its rows
    /// and places it.
    pub(crate) fn menu_panel(&self, id: &'static str) -> Stateful<Div> {
        self.menu().panel(
            div()
                .id(id)
                .debug_selector(move || id.into())
                .p(self.at(MENU_PADDING))
                .flex()
                .flex_col(),
        )
    }

    /// A row of a view's own menu that picks one value: highlighted under the
    /// pointer or the keys, with the check at its end on the value chosen.
    /// The caller adds its handlers.
    pub(crate) fn choice(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        label: impl Into<SharedString>,
        highlighted: bool,
        chosen: bool,
    ) -> Stateful<Div> {
        let menu = self.menu();
        let check = format!("{selector}-check");
        let label_selector = format!("{selector}-label");
        menu.row(
            id,
            selector,
            Leading::Empty,
            ItemState {
                selected: highlighted,
                ..ItemState::default()
            },
        )
        .child(menu.label(label_selector, label.into()))
        .when(chosen, |row| {
            row.child(
                svg()
                    .debug_selector(move || check)
                    .path("icons/check.svg")
                    .size(self.at(MENU_CHECK))
                    .flex_none()
                    .text_color(self.accent_ink),
            )
        })
    }

    /// A segmented control's box; its segments are [`Self::segment`].
    pub(crate) fn segments(&self, selector: String) -> Div {
        div()
            .debug_selector(move || selector)
            .flex_none()
            .h(self.at(CONTROL))
            .flex()
            .items_center()
            .gap(self.at(SEGMENT_INSET))
            // The inset includes the box's 1 px ring.
            .p(self.at(SEGMENT_INSET) - px(1.))
            .rounded(self.at(CONTROL_RADIUS))
            .border_1()
            .border_color(self.line)
            .bg(self.side)
    }

    /// One segment: its label or icon, filled while chosen. The caller adds
    /// its click; an icon's segment narrows its padding.
    pub(crate) fn segment(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        label: impl IntoElement,
        on: bool,
    ) -> Stateful<Div> {
        let hover = self.text;
        div()
            .id(id)
            .debug_selector(move || selector)
            .h_full()
            .px(self.at(SEGMENT_PADDING))
            .flex()
            .items_center()
            .gap(self.at(CONTROL_GAP))
            .rounded(self.at(SEGMENT_RADIUS))
            .whitespace_nowrap()
            .text_size(px(self.scale.name - 0.5))
            .text_color(if on { self.title } else { self.muted })
            .when(on, |segment| {
                segment.bg(self.selection).font_weight(FontWeight::MEDIUM)
            })
            .when(!on, |segment| {
                segment
                    .cursor_pointer()
                    .hover(move |segment| segment.text_color(hover))
            })
            .child(label)
    }

    /// A filter that turns on and off: ringed while off, washed with the
    /// accent behind a check while on. The ring keeps its width either way,
    /// so nothing moves. The caller adds its click.
    pub(crate) fn pill(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        label: &'static str,
        on: bool,
    ) -> Stateful<Div> {
        let hover = self.hover;
        let mark = format!("{selector}-check");
        div()
            .id(id)
            .debug_selector(move || selector)
            .flex_none()
            .h(self.at(CONTROL))
            .px(self.at(CONTROL_PADDING))
            .flex()
            .items_center()
            .gap(self.at(CONTROL_GAP))
            .rounded(self.at(CONTROL_RADIUS))
            .border_1()
            .whitespace_nowrap()
            .cursor_pointer()
            .text_size(self.control_text())
            .when(on, |pill| {
                pill.bg(self.pill)
                    .border_color(self.pill)
                    .text_color(self.accent_ink)
                    .child(
                        svg()
                            .debug_selector(move || mark)
                            .path("icons/check.svg")
                            .size(self.at(PILL_MARK))
                            .flex_none()
                            .text_color(self.accent_ink),
                    )
            })
            .when(!on, |pill| {
                pill.border_color(self.line)
                    .text_color(self.text)
                    .hover(move |pill| pill.bg(hover))
            })
            .child(label)
    }

    /// The size of a pill's and a dropdown's text.
    pub(crate) fn control_text(&self) -> Pixels {
        px(self.scale.name - 0.5)
    }

    /// The space between a filter row's controls.
    pub(crate) fn control_gap(&self) -> f32 {
        f32::from(self.at(CONTROL_GAP))
    }

    /// A filter row's inset on each side.
    pub(crate) fn filters_padding(&self) -> f32 {
        f32::from(self.at(FILTERS_PADDING))
    }

    /// How wide [`Self::pill`] draws around a label `text` wide.
    pub(crate) fn pill_width(&self, text: f32, on: bool) -> f32 {
        let mark = if on {
            f32::from(self.at(PILL_MARK) + self.at(CONTROL_GAP))
        } else {
            0.
        };
        text + mark + f32::from(self.at(CONTROL_PADDING)) * 2. + 2.
    }

    /// How wide [`Self::dropdown`] draws with its icon and a value `text`
    /// wide, and no prefix.
    pub(crate) fn dropdown_width(&self, text: f32) -> f32 {
        let glyphs = f32::from(self.at(DROPDOWN_ICON)) * 2. + f32::from(self.at(CONTROL_GAP)) * 2.;
        text + glyphs + f32::from(self.at(CONTROL_PADDING) + self.at(DROPDOWN_PADDING_RIGHT)) + 2.
    }

    /// A small dropdown's button: an icon, a quiet prefix, the value and a
    /// chevron. The menu it opens is drawn by whoever opens it.
    pub(crate) fn dropdown(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        icon: Option<&'static str>,
        prefix: Option<&'static str>,
        value: impl Into<SharedString>,
    ) -> Stateful<Div> {
        let hover = self.settings().raised();
        let glyph = |path: &'static str| {
            svg()
                .path(path)
                .size(self.at(DROPDOWN_ICON))
                .flex_none()
                .text_color(self.muted)
        };
        div()
            .id(id)
            .debug_selector(move || selector)
            .flex_none()
            .h(self.at(CONTROL))
            .pl(self.at(CONTROL_PADDING))
            .pr(self.at(DROPDOWN_PADDING_RIGHT))
            .flex()
            .items_center()
            .gap(self.at(CONTROL_GAP))
            .rounded(self.at(CONTROL_RADIUS))
            .bg(self.side)
            .border_1()
            .border_color(self.line)
            .whitespace_nowrap()
            .cursor_pointer()
            .text_size(self.control_text())
            .text_color(self.title)
            .hover(move |dropdown| dropdown.border_color(hover))
            .when_some(icon, |dropdown, icon| dropdown.child(glyph(icon)))
            .when_some(prefix, |dropdown, prefix| {
                dropdown.child(div().flex_none().text_color(self.muted).child(prefix))
            })
            // Where the caller lets the dropdown narrow, its value ends in
            // an ellipsis.
            .child(div().min_w_0().truncate().child(value.into()))
            .child(glyph("icons/chevron-down.svg"))
    }

    /// A text field's frame at a bar's height, its icon first. The caller
    /// adds the input and anything after it.
    pub(crate) fn field(&self, selector: String, focused: bool, icon: Option<&'static str>) -> Div {
        self.settings()
            .field(selector, focused)
            .min_w_0()
            .h(self.at(FIELD))
            .when_some(icon, |field, icon| {
                field.child(
                    svg()
                        .path(icon)
                        .size(self.at(ICON))
                        .flex_none()
                        .text_color(self.muted),
                )
            })
    }

    /// The row under a list's tabs that holds its search and filters.
    pub(crate) fn filters(&self, selector: &'static str) -> Div {
        div()
            .debug_selector(move || selector.into())
            .flex_none()
            .min_w_0()
            .h(self.at(FILTERS))
            .px(self.at(FILTERS_PADDING))
            .flex()
            .items_center()
            .gap(self.at(CONTROL_GAP))
            .overflow_hidden()
    }

    /// A list's column headings, lined up with its rows' text.
    pub(crate) fn columns(&self, selector: &'static str) -> Div {
        div()
            .debug_selector(move || selector.into())
            .flex_none()
            .min_w_0()
            .h(self.at(COLUMNS))
            .px(self.at(COLUMNS_PADDING))
            .flex()
            .items_center()
            .gap(self.at(ROW_GAP))
            .whitespace_nowrap()
            .text_size(px(self.scale.detail))
            .font_weight(FontWeight::MEDIUM)
            .text_color(self.muted)
    }

    /// The column a list's rows sit in, kept off its sides.
    pub(crate) fn list(&self) -> Div {
        div().min_w_0().flex().flex_col().px(self.at(LIST_INSET))
    }

    /// A list's row, one line or two: rounded, filled while hovered as the
    /// menus' rows and while selected as the sidebar's. The caller adds its
    /// content and handlers.
    pub(crate) fn row(
        &self,
        id: impl Into<ElementId>,
        selector: String,
        two_lines: bool,
        selected: bool,
    ) -> Stateful<Div> {
        let hover = self.hover;
        div()
            .id(id)
            .debug_selector(move || selector)
            .flex_none()
            .min_w_0()
            .h(px(self.row_height(two_lines)))
            .px(self.at(ROW_PADDING))
            .flex()
            .items_center()
            .gap(self.at(ROW_GAP))
            .rounded(px(SELECTION_RADIUS))
            .whitespace_nowrap()
            .cursor_pointer()
            .text_size(px(self.scale.name))
            .when(selected, |row| {
                row.bg(self.selection).text_color(self.title)
            })
            .when(!selected, |row| {
                row.text_color(self.text).hover(move |row| row.bg(hover))
            })
    }

    /// A panel beside a view's content, a step darker than it, its hairline
    /// toward the content.
    pub(crate) fn side_panel(&self, selector: &'static str, edge: Edge) -> Div {
        let panel = div()
            .debug_selector(move || selector.into())
            .flex_none()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(self.side)
            .border_color(self.line);
        match edge {
            Edge::Left => panel.border_r_1(),
            Edge::Right => panel.border_l_1(),
        }
    }

    /// A side panel's heading: its title and, if given, a count in a chip.
    /// The caller adds its controls, which sit at the right.
    pub(crate) fn side_header(
        &self,
        selector: &'static str,
        title: &'static str,
        count: Option<usize>,
    ) -> Div {
        self.heading(selector, SIDE_HEADER, SIDE_PADDING_RIGHT)
            .child(
                div()
                    .flex_none()
                    .text_size(px(self.scale.name - 0.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.title)
                    .child(title),
            )
            .when_some(count, |header, count| {
                header.child(self.settings().chip(
                    format!("{selector}-count"),
                    count.to_string(),
                    Tone::Neutral,
                ))
            })
            .child(div().flex_1())
    }

    /// A card's title row. The caller adds what follows the title, and a
    /// spacer before any controls that sit at the right.
    pub(crate) fn card_header(&self, selector: &'static str, title: &'static str) -> Div {
        self.heading(selector, CARD_HEADER, CARD_PADDING_RIGHT)
            .child(
                div()
                    .flex_none()
                    .text_size(px(self.scale.name))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(self.title)
                    .child(title),
            )
    }

    fn heading(&self, selector: &'static str, height: f32, right: f32) -> Div {
        div()
            .debug_selector(move || selector.into())
            .flex_none()
            .min_w_0()
            .h(self.at(height))
            .pl(self.at(SIDE_PADDING_LEFT))
            .pr(self.at(right))
            .flex()
            .items_center()
            .gap(self.at(HEADER_GAP))
            .whitespace_nowrap()
    }

    /// What a view shows with nothing in it, centred in its room: the
    /// view's icon on a tile of the accent, when there is room for one, and
    /// the text, which `selector` names; the room is `{selector}-area` and
    /// the tile `{selector}-tile`. The caller adds its actions below.
    pub(crate) fn empty(
        &self,
        selector: impl Into<String>,
        icon: Option<&'static str>,
        text: impl Into<SharedString>,
    ) -> Div {
        let selector = selector.into();
        let (area, tile) = (format!("{selector}-area"), format!("{selector}-tile"));
        div()
            .debug_selector(move || area)
            .size_full()
            .min_h_0()
            .p(self.at(EMPTY_PADDING))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(self.at(EMPTY_GAP))
            .when_some(icon, |empty, icon| {
                empty.child(
                    div()
                        .debug_selector(move || tile)
                        .flex_none()
                        .size(self.at(EMPTY_TILE))
                        .rounded(self.at(EMPTY_RADIUS))
                        .bg(self.tile)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            svg()
                                .path(icon)
                                .size(self.at(EMPTY_ICON))
                                .text_color(self.accent_ink),
                        ),
                )
            })
            .child(
                div()
                    .debug_selector(move || selector)
                    .text_center()
                    .text_size(px(self.scale.name))
                    .text_color(self.text)
                    .child(text.into()),
            )
    }

    /// An agent's state as the sidebar shows it: a dot in its colour, hollow
    /// at rest.
    pub(crate) fn status_dot(&self, colour: u32, hollow: bool) -> Div {
        let dot = div().flex_none().size(self.at(STATUS_DOT)).rounded_full();
        if hollow {
            dot.border(self.at(STATUS_RING)).border_color(rgb(colour))
        } else {
            dot.bg(rgb(colour))
        }
    }
}

#[cfg(test)]
mod tests;
