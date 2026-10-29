//! The agent activity chart: working and blocked agents on every device for
//! the last two hours as stacked bars, and, behind "View all devices", one
//! lane per device on the same time axis. Bars and lanes paint as quads on a
//! canvas, so two hours of steps on many devices stay a handful of elements.

use super::{
    Device, Tally,
    history::{Counts, History, STEPS},
    view::Look,
};
use crate::{
    HerdrWindow,
    browser::{Slot, TabId},
    config::mix,
    design::{ButtonKind, ViewLook},
};
use gpui::{prelude::*, *};
use herdr_client::protocol::AgentStatus;

const BARS_HEIGHT: f32 = 64.;
/// The card's padding and its title row, at `[ui] size = 12`.
const CARD_PADDING: f32 = 14.;
const CARD_HEADER: f32 = 44.;
const LANE_HEIGHT: f32 = 12.;
/// The labels left of the lanes and the counts right of them, so the bars
/// and every lane share one time axis.
const LABEL_WIDTH: f32 = 150.;
const NOW_WIDTH: f32 = 72.;
/// Space between two steps, when the chart is wide enough for it.
const STEP_GAP: f32 = 1.;
/// The gap between a row's columns (`gap_3`).
const COLUMN_GAP: f32 = 12.;
/// The card's narrowest with the figure beside the bars, and the bars'
/// narrowest with the axis's middle label, at `[ui] size = 12`.
const CHART_SIDE_MIN: f32 = 520.;
const AXIS_MIDDLE_MIN: f32 = 260.;

/// How the activity card lays out at its width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Shape {
    /// The figure and the counts beside the bars, or over them.
    beside: bool,
    /// Whether the axis has room for "1 h ago" between its ends.
    middle: bool,
}

impl Shape {
    /// The card's shape at `card` wide, or wide before its first layout.
    pub(super) fn of(view: ViewLook, card: Option<f32>) -> Self {
        let Some(card) = card else {
            return Self {
                beside: true,
                middle: true,
            };
        };
        let beside = card >= f32::from(view.at(CHART_SIDE_MIN));
        let inner = card - card_chrome(view);
        let bars = if beside { inner - BESIDE } else { inner };
        Self {
            beside,
            middle: bars >= f32::from(view.at(AXIS_MIDDLE_MIN)),
        }
    }

    /// The card widths at which [`Self::of`] changes: the figure goes
    /// beside, and the middle label shows beside and under.
    pub(super) fn cuts(view: ViewLook) -> [f32; 3] {
        let middle = f32::from(view.at(AXIS_MIDDLE_MIN)) + card_chrome(view);
        [f32::from(view.at(CHART_SIDE_MIN)), middle + BESIDE, middle]
    }
}

/// The room the figure's and the counts' columns take beside the bars.
const BESIDE: f32 = LABEL_WIDTH + NOW_WIDTH + 2. * COLUMN_GAP;

/// The card's padding and its border, on both sides.
fn card_chrome(view: ViewLook) -> f32 {
    f32::from(view.at(CARD_PADDING)) * 2. + 2.
}

/// The full two hours, oldest first: steps the window has not seen yet are
/// None on the left, so the axis means the same from the first minute.
pub(super) fn padded<T>(values: Vec<T>) -> Vec<Option<T>> {
    let slots = STEPS + 1;
    let skip = values.len().saturating_sub(slots);
    let missing = slots.saturating_sub(values.len());
    std::iter::repeat_with(|| None)
        .take(missing)
        .chain(values.into_iter().skip(skip).map(Some))
        .collect()
}

/// Each step's slot within `bounds`, left to right.
fn slots(bounds: Bounds<Pixels>, count: usize) -> impl Iterator<Item = (usize, Pixels, Pixels)> {
    let width = f32::from(bounds.size.width) / count.max(1) as f32;
    let gap = if width > 3. * STEP_GAP { STEP_GAP } else { 0. };
    (0..count).map(move |index| {
        let left = bounds.origin.x + px(index as f32 * width);
        (index, left, px((width - gap).max(0.5)))
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn activity(
    look: &Look,
    history: &History,
    shown: &[&Device],
    now: Tally,
    shape: Shape,
    lanes: bool,
    slot: Slot,
    id: TabId,
    cx: &mut Context<HerdrWindow>,
) -> Div {
    let theme = look.theme;
    let totals = history.totals();
    let peak = totals
        .iter()
        .map(|counts| counts.working + counts.blocked)
        .max()
        .unwrap_or(0);
    let working = look.status(AgentStatus::Working);
    let blocked = look.status(AgentStatus::Blocked);
    let view = look.view;
    // Beside: a label, the bars and a count share a row. Narrow: the label
    // and the count share a line, and the bars get the card's width under.
    let row = |left: Div, middle: AnyElement, right: Option<Div>| {
        if shape.beside {
            return div()
                .flex()
                .items_center()
                .gap(px(COLUMN_GAP))
                .child(div().flex_none().w(px(LABEL_WIDTH)).min_w_0().child(left))
                .child(div().flex_1().min_w_0().child(middle))
                .child(div().flex_none().w(px(NOW_WIDTH)).children(right));
        }
        div()
            .flex()
            .flex_col()
            .gap(view.at(6.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(COLUMN_GAP))
                    .child(div().flex_1().min_w_0().child(left))
                    .children(right.map(|right| div().flex_none().child(right))),
            )
            .child(middle)
    };
    let summary = div()
        .debug_selector(|| "devices-summary".into())
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(look.ui.size + 9.))
                .font_weight(FontWeight::SEMIBOLD)
                .child(now.working.to_string()),
        )
        .child(
            div()
                .text_size(look.small())
                .text_color(rgb(theme.subtext()))
                .child("working now"),
        )
        .child(
            div()
                .text_size(look.small())
                .text_color(rgb(theme.muted))
                .child(format!("peak {peak} in 2 h")),
        );
    let toggle = view
        .button(
            SharedString::from(slot.selector("devices-lanes-toggle")),
            slot.selector("devices-lanes-toggle"),
            None,
            if lanes {
                "Hide devices"
            } else {
                "View all devices"
            },
            ButtonKind::Normal,
            true,
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            if let Some(page) = this.devices_overview.pages.get_mut(&id) {
                page.lanes = !page.lanes;
                cx.notify();
            }
        }));
    // Its ends always; the middle only where it fits whole between them.
    let axis = div().w_full().flex().justify_between().children(
        ["2 h ago", "1 h ago", "now"]
            .into_iter()
            .enumerate()
            .filter(|(index, _)| *index != 1 || shape.middle)
            .map(|(index, label)| {
                look.mono(label)
                    .debug_selector(move || format!("devices-axis-{index}"))
            }),
    );
    let body = div()
        .px(view.at(CARD_PADDING))
        .pb(view.at(CARD_PADDING))
        .flex()
        .flex_col()
        .gap(view.at(8.))
        .child(row(
            summary,
            div()
                .debug_selector(|| "devices-bars".into())
                .w_full()
                .child(bars(
                    padded(totals),
                    peak,
                    [working, blocked],
                    theme,
                    BARS_HEIGHT,
                ))
                .into_any_element(),
            None,
        ))
        .when(lanes, |body| {
            body.child(div().h(px(1.)).bg(view.line()))
                .children(shown.iter().map(|device| {
                    let lane = history.lane(&device.id);
                    row(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(look.link(device.link))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(look.small())
                                    .child(device.label.clone()),
                            ),
                        lane_canvas(padded(lane), working, blocked, theme),
                        Some(look.mono(match device.link {
                            super::Link::Online => {
                                format!("{}/{} now", device.tally.working, device.tally.total())
                            }
                            super::Link::Connecting => "connecting".into(),
                            super::Link::Offline => "offline".into(),
                            super::Link::Disabled => "disabled".into(),
                        })),
                    )
                }))
        })
        .map(|body| {
            // The axis lines up with the bars.
            if shape.beside {
                body.child(row(div(), axis.into_any_element(), None))
            } else {
                body.child(axis)
            }
        });
    view.settings()
        .card(slot.selector("devices-activity"))
        .child(
            view.card_header("devices-activity-header", "Agent activity")
                // A narrow tab wraps the legend and the button under the
                // title rather than cutting them.
                .h_auto()
                .min_h(view.at(CARD_HEADER))
                .py(view.at(6.))
                .flex_wrap()
                .whitespace_normal()
                .child(look.mono("last 2 h"))
                .child(div().flex_1())
                .child(look.legend())
                .child(toggle),
        )
        .child(body)
}

/// Every device's agents for the last two hours as a small chart, for the
/// device picker's activity card.
pub(crate) fn sparkline(history: &History, look: &Look, height: f32) -> AnyElement {
    let totals = history.totals();
    let peak = totals
        .iter()
        .map(|counts| counts.working + counts.blocked)
        .max()
        .unwrap_or(0);
    bars(
        padded(totals),
        peak,
        [
            look.status(AgentStatus::Working),
            look.status(AgentStatus::Blocked),
        ],
        look.theme,
        height,
    )
}

/// Working agents stacked under blocked ones, scaled to the busiest step.
/// A faint baseline spans the whole two hours, so the axis reads as time
/// even before the window has seen most of it.
fn bars(
    steps: Vec<Option<Counts>>,
    peak: u16,
    [working, blocked]: [u32; 2],
    theme: &crate::config::Theme,
    height: f32,
) -> AnyElement {
    let count = steps.len();
    let (surface, baseline) = (theme.surface, theme.active);
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            window.paint_quad(fill(
                Bounds::new(
                    point(bounds.origin.x, bounds.bottom() - px(1.)),
                    size(bounds.size.width, px(1.)),
                ),
                rgb(baseline),
            ));
            let scale = f32::from(bounds.size.height) / f32::from(peak.max(1));
            for (index, left, width) in slots(bounds, count) {
                let Some(counts) = steps[index] else {
                    continue;
                };
                // Past steps are quieter than the one still counting.
                let soften = |color: u32| {
                    if index + 1 == count {
                        color
                    } else {
                        mix(surface, color, 70)
                    }
                };
                let bottom = bounds.bottom();
                let working_height = px(f32::from(counts.working) * scale);
                let blocked_height = px(f32::from(counts.blocked) * scale);
                if counts.working > 0 {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(left, bottom - working_height),
                            size(width, working_height),
                        ),
                        rgb(soften(working)),
                    ));
                }
                if counts.blocked > 0 {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(left, bottom - working_height - blocked_height),
                            size(width, blocked_height),
                        ),
                        rgb(soften(blocked)),
                    ));
                }
            }
        },
    )
    .w_full()
    .h(px(height))
    .into_any_element()
}

/// One device's steps as cells: darker the more agents worked, blocked
/// where one waited, faint where nothing ran, empty while not connected.
fn lane_canvas(
    steps: Vec<Option<Option<Counts>>>,
    working: u32,
    blocked: u32,
    theme: &crate::config::Theme,
) -> AnyElement {
    let count = steps.len();
    let (surface, idle) = (theme.surface, theme.active);
    let busiest = steps
        .iter()
        .flatten()
        .flatten()
        .map(|counts| counts.working)
        .max()
        .unwrap_or(0)
        .max(1);
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            for (index, left, width) in slots(bounds, count) {
                let Some(Some(counts)) = steps[index] else {
                    continue;
                };
                let color = if counts.blocked > 0 {
                    blocked
                } else if counts.working == 0 {
                    idle
                } else {
                    let share = 30 + 70 * u32::from(counts.working) / u32::from(busiest);
                    mix(surface, working, share.min(100))
                };
                window.paint_quad(fill(
                    Bounds::new(
                        point(left, bounds.origin.y),
                        size(width, bounds.size.height),
                    ),
                    rgb(color),
                ));
            }
        },
    )
    .w_full()
    .h(px(LANE_HEIGHT))
    .into_any_element()
}
