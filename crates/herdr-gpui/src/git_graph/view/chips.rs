//! Branch and tag labels in the label column, as hgg draws them: pills in
//! the lane's colour with GitKraken's icons, folded into a `+N` badge when
//! they do not fit, and shown whole while the pointer is over one.

use super::GitGraphView;
use crate::{
    config::mix,
    git_graph::{
        chips::{Chip, ChipFit, ChipKind, fit_chips},
        geometry::{
            CHIP_GAP, CHIP_H, CHIP_PAD, CHIP_TEXT, ICON, ICON_STEP, MIN_LEADER, REFS_INSET, ROW_H,
        },
    },
};
use gpui::{prelude::*, *};

const CHECK: &str = "icons/graph-check.svg";
const LAPTOP: &str = "icons/graph-laptop.svg";
const REMOTE: &str = "icons/graph-remote.svg";
const TAG: &str = "icons/graph-tag.svg";
const STASH: &str = "icons/graph-stash.svg";
/// Room the `+N` badge adds around its text.
const BADGE_PAD: f32 = 12.;
/// Shaped widths kept before the cache starts again.
const WIDTHS_KEPT: usize = 4096;
/// The line from the labels to the graph, at this opacity.
pub(super) const LEADER_ALPHA: f32 = 0.55;

/// A label's icons in GitKraken's order: ✓, tag or stash before the name,
/// local and remote after it.
pub(crate) fn chip_icons(chip: &Chip) -> (Vec<&'static str>, Vec<&'static str>) {
    let mut before = Vec::new();
    let mut after = Vec::new();
    if chip.current {
        before.push(CHECK);
    }
    if chip.kind == ChipKind::Branch {
        after.push(LAPTOP);
    }
    if chip.kind == ChipKind::Remote || !chip.remotes.is_empty() {
        after.push(REMOTE);
    }
    match chip.kind {
        ChipKind::Tag => before.push(TAG),
        ChipKind::Stash => before.push(STASH),
        ChipKind::Detached | ChipKind::Branch | ChipKind::Remote => {}
    }
    (before, after)
}

/// What a label says on hover: every ref it stands for.
fn chip_hint(chip: &Chip) -> String {
    let mut text = chip.names.join("\n");
    if chip.current && chip.kind == ChipKind::Branch {
        text.push_str("\n(current branch, HEAD)");
    }
    if !chip.remotes.is_empty() && chip.kind != ChipKind::Remote {
        text.push_str(&format!("\nAlso on: {}", chip.remotes.join(", ")));
    }
    text
}

impl GitGraphView {
    /// The shaped width of `text` in the labels' font, cached.
    fn text_width(&mut self, text: &str, window: &Window) -> f32 {
        if let Some(width) = self.chip_widths.get(text) {
            return *width;
        }
        if self.chip_widths.len() >= WIDTHS_KEPT {
            self.chip_widths.clear();
        }
        let mut font = self.look.ui.font();
        font.weight = FontWeight::MEDIUM;
        let run = TextRun {
            len: text.len(),
            font,
            color: black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let text = SharedString::from(text.to_owned());
        let width = f32::from(
            window
                .text_system()
                .shape_line(text.clone(), px(CHIP_TEXT), &[run], None)
                .width,
        )
        .ceil();
        self.chip_widths.insert(text, width);
        width
    }

    fn chip_width(&mut self, chip: &Chip, window: &Window) -> f32 {
        let (before, after) = chip_icons(chip);
        self.text_width(&chip.label, window)
            + 2. * CHIP_PAD
            + (before.len() + after.len()) as f32 * ICON_STEP
    }

    pub(super) fn fit(&mut self, chips: &[Chip], window: &Window) -> ChipFit {
        let widths: Vec<f32> = chips
            .iter()
            .map(|chip| self.chip_width(chip, window))
            .collect();
        let badges: Vec<f32> = (0..=chips.len())
            .map(|hidden| self.text_width(&format!("+{hidden}"), window) + BADGE_PAD)
            .collect();
        fit_chips(
            &widths,
            self.refs_width() - REFS_INSET,
            |hidden| badges.get(hidden).copied().unwrap_or_default(),
            CHIP_GAP,
            MIN_LEADER,
        )
    }

    /// `(fill, text, border, bold)` for a label on a lane of `lane` colour.
    fn chip_colours(&self, chip: &Chip, lane: u32) -> (Option<u32>, u32, Option<u32>, bool) {
        let theme = &self.look.theme;
        let fill = |percent| mix(theme.background, lane, percent);
        let (fill, bold) = match chip.kind {
            ChipKind::Stash => return (None, theme.subtext(), Some(lane), false),
            _ if chip.current || chip.kind == ChipKind::Detached => (fill(88), true),
            ChipKind::Remote => (fill(26), false),
            _ => (fill(40), false),
        };
        (Some(fill), theme.text_on(fill), None, bold)
    }

    fn chip(&self, chip: &Chip, width: f32, lane: u32) -> Div {
        let (fill, text, border, bold) = self.chip_colours(chip, lane);
        let (before, after) = chip_icons(chip);
        let icon = |path: &'static str| {
            svg()
                .flex_none()
                .size(px(ICON))
                .path(path)
                .text_color(rgb(text))
        };
        div()
            .flex_none()
            .w(px(width))
            .h(px(CHIP_H))
            .px(px(CHIP_PAD))
            .rounded(px(3.))
            .flex()
            .items_center()
            .gap(px(ICON_STEP - ICON))
            .overflow_hidden()
            .when_some(fill, |el, fill| el.bg(rgb(fill)))
            .when_some(border, |el, border| el.border_1().border_color(rgb(border)))
            .text_color(rgb(text))
            .text_size(px(CHIP_TEXT))
            .line_height(px(CHIP_H))
            .font_weight(if bold {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::MEDIUM
            })
            .children(before.into_iter().map(icon))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .child(chip.label.clone()),
            )
            .children(after.into_iter().map(icon))
    }

    /// The label column of `row`, and the hovered label shown whole, which
    /// the row draws above everything else.
    pub(super) fn render_refs(
        &mut self,
        row: usize,
        chips: &[Chip],
        lane: u32,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> (AnyElement, Option<AnyElement>) {
        let fit = self.fit(chips, window);
        let theme = self.look.theme.clone();
        let mut cell = div()
            .id(("graph-refs", row))
            .flex_none()
            .w(px(self.refs_width()))
            .h_full()
            .flex()
            .items_center()
            .gap(px(CHIP_GAP))
            .pl(px(REFS_INSET))
            .overflow_hidden();
        let mut whole = None;
        let mut x = REFS_INSET;
        for (position, &index) in fit.shown.iter().enumerate() {
            let Some(chip) = chips.get(index) else {
                continue;
            };
            let natural = self.chip_width(chip, window);
            let width = match fit.first_max_width {
                Some(max) if position == 0 => max.min(natural),
                _ => natural,
            };
            let hint: SharedString = chip_hint(chip).into();
            cell = cell.child(
                self.chip(chip, width, lane)
                    .id(SharedString::from(format!("graph-chip-{row}-{position}")))
                    .debug_selector(move || format!("graph-chip-{row}-{position}"))
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        let key = Some((row, position));
                        if *hovered {
                            this.hovered_chip = key;
                        } else if this.hovered_chip == key {
                            this.hovered_chip = None;
                        }
                        cx.notify();
                    }))
                    .tooltip(self.look.view().menu().tooltip(hint)),
            );
            if self.hovered_chip == Some((row, position)) && width < natural {
                whole = Some(
                    deferred(
                        div()
                            .absolute()
                            .left(px(x))
                            .top(px((ROW_H - CHIP_H) / 2.))
                            .shadow_md()
                            .child(self.chip(chip, natural, lane)),
                    )
                    .into_any_element(),
                );
            }
            x += width + CHIP_GAP;
        }
        if fit.hidden > 0 {
            let hidden: Vec<&str> = chips
                .iter()
                .enumerate()
                .filter(|(index, _)| !fit.shown.contains(index))
                .flat_map(|(_, chip)| chip.names.iter().map(String::as_str))
                .collect();
            let hint: SharedString = hidden.join("\n").into();
            cell = cell.child(
                div()
                    .id(SharedString::from(format!("graph-chip-more-{row}")))
                    .debug_selector(move || format!("graph-chip-more-{row}"))
                    .flex_none()
                    .h(px(CHIP_H))
                    .px(px(BADGE_PAD / 2.))
                    .rounded(px(3.))
                    .bg(rgb(mix(theme.background, theme.foreground, 8)))
                    .text_color(rgb(theme.subtext()))
                    .text_size(px(CHIP_TEXT))
                    .line_height(px(CHIP_H))
                    .child(format!("+{}", fit.hidden))
                    .tooltip(self.look.view().menu().tooltip(hint)),
            );
        }
        let mut leader: Hsla = rgb(lane).into();
        leader.a = LEADER_ALPHA;
        cell = cell.child(div().flex_1().min_w(px(12.)).h(px(1.5)).bg(leader));
        (cell.into_any_element(), whole)
    }
}
