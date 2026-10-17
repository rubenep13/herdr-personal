//! The Tree layout's host header: where the host runs, its name and kind, a
//! connection dot, and its CPU and memory, which give way to fold and new
//! space buttons while the header is pointed at.

use super::{label_text, layout::SidebarLook, layouts::control, row::removing_dot, wash};
use crate::{
    HerdrWindow,
    config::{Theme, TreeHierarchy},
    controls::Command,
    design::{ROW_HEIGHT, TypeScale},
    endpoint::{Endpoint, origin::Origin},
    state::ConnectionStatus,
    system_load::Reading,
};
use gpui::{prelude::*, *};

/// Hover group of a host header, apart from the rows' so pointing at a row
/// never reveals its host's buttons.
const HOST_GROUP: &str = "sidebar-host";
const ORIGIN: f32 = 16.;
const DOT: f32 = 7.;

/// The connection dot's colour: the online green while connected, muted
/// otherwise.
pub(super) fn host_dot(status: &ConnectionStatus, theme: &Theme) -> u32 {
    if status.is_connected() {
        crate::menu::online(theme)
    } else {
        theme.muted
    }
}

fn icon(path: &'static str, size: f32, color: u32) -> Svg {
    svg()
        .path(path)
        .size(px(size))
        .flex_none()
        .text_color(rgb(color))
}

/// How the host is reached, in a word or two.
fn kind(origin: Origin) -> String {
    match origin {
        Origin::Local => "This machine".into(),
        Origin::Ssh => "SSH".into(),
        Origin::Tailscale => "Tailscale".into(),
        Origin::Wsl => "WSL".into(),
        #[cfg(feature = "cloud")]
        Origin::Cloud { provider, .. } => crate::cloud::name(provider).into(),
    }
}

/// `12% CPU  50% mem` in figures of one width, so the text holds still as
/// the numbers change.
fn load(selector: String, reading: &Reading, size: f32) -> Option<Div> {
    let sample = reading.latest()?;
    let cpu = sample.cpu.map(|cpu| format!("{cpu:.0}% CPU"));
    let memory = sample
        .memory
        .map(|memory| format!("{:.0}% mem", memory.percent()));
    if cpu.is_none() && memory.is_none() {
        return None;
    }
    let part = |text: String| div().flex_none().child(label_text(&text));
    Some(
        div()
            .debug_selector(move || selector)
            .flex()
            .flex_shrink(1000.)
            .min_w_0()
            .overflow_hidden()
            .gap(px(8.))
            .text_size(px(size))
            .font_features(FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)])))
            .children(cpu.map(part))
            .children(memory.map(part))
            .group_hover(HOST_GROUP, |style| style.hidden()),
    )
}

impl HerdrWindow {
    /// [`Self::host_row`] for the Tree layout, with the same contract: the
    /// pinned copy takes `sticky-` ids and an opaque background. The header
    /// marks no selection: the selected row below it already does.
    pub(super) fn tree_host_row(
        &self,
        endpoint: &Endpoint,
        _selected: bool,
        look: SidebarLook,
        _width: f32,
        pinned: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = &self.theme;
        let scale = TypeScale::of(&self.config.ui);
        let prefix = if pinned { "sticky-" } else { "" };
        let id = endpoint.id.clone();
        let live = self
            .endpoints
            .iter()
            .position(|other| other.id == endpoint.id)
            .filter(|&index| index == self.selected_endpoint)
            .map_or(&endpoint.live, |_| &self.live);
        let connected = live.status.is_connected();
        let removing = self.menu.removing_devices.contains(&endpoint.id);
        let origin = Origin::of(&endpoint.connection.target);
        // A cloud machine has no host to read its load from, and a host that
        // is not connected has no current load to show.
        let reading = crate::usage::Host::of(&endpoint.connection.target)
            .filter(|_| self.config.show_system_load && connected)
            .and_then(|host| self.system_load.get(&host));
        let text = if endpoint.enabled {
            theme.foreground
        } else {
            theme.muted
        };
        let hover = Hsla::from(rgb(theme.active)).opacity(look.mark * 0.6);
        let (collapse_id, new_id, select_id, menu_id) =
            (id.clone(), id.clone(), id.clone(), id.clone());
        let fold = control(
            SharedString::from(format!("{prefix}collapse-host-{id}")).into(),
            format!("{prefix}host-fold-{id}"),
            (false, HOST_GROUP),
            theme,
        )
        .child(icon(
            if endpoint.collapsed {
                "icons/chevron-right.svg"
            } else {
                "icons/chevron-down.svg"
            },
            14.,
            theme.muted,
        ))
        .on_click(cx.listener(move |this, _, _, cx| {
            cx.stop_propagation();
            if let Some(endpoint) = this.endpoints.iter_mut().find(|e| e.id == collapse_id) {
                endpoint.collapsed = !endpoint.collapsed;
                let collapsed = endpoint.collapsed;
                // Folds are kept across restarts, by endpoint ID.
                crate::sidebar::tree::storage::TreeState::update(cx, |tree| {
                    (tree.set_host_collapsed(&collapse_id, collapsed), ())
                });
            }
            cx.notify();
        }));
        let new_space = control(
            SharedString::from(format!("{prefix}host-new-space-{id}")).into(),
            format!("{prefix}host-new-space-{id}"),
            (false, HOST_GROUP),
            theme,
        )
        .child(icon("icons/plus.svg", 14., theme.muted))
        .on_click(cx.listener(move |this, _, window, cx| {
            cx.stop_propagation();
            if this.select_endpoint(&new_id, cx) {
                this.command(Command::Workspace, window, cx);
            }
        }));
        let carried = !pinned
            && self
                .device_drag
                .as_ref()
                .is_some_and(|drag| drag.carries(&endpoint.id));
        div()
            .id(SharedString::from(format!("{prefix}host-{id}")))
            .debug_selector(|| format!("{prefix}host-{id}"))
            .group(HOST_GROUP)
            .h(px(ROW_HEIGHT))
            .flex_none()
            .relative()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(look.content_x()))
            // Rows scrolling underneath show through anything translucent, so
            // the copy paints the sidebar's own background, and stops hover
            // and clicks but not the wheel reaching the rows beneath.
            .when(pinned, |row| {
                // On a block, the copy is the block's top.
                let row = if look.tree.hierarchy == TreeHierarchy::Block {
                    super::render::pinned_block(row, theme)
                } else {
                    row.bg(rgb(theme.sidebar_background()))
                };
                row.block_mouse_except_scroll()
            })
            .map(|row| {
                wash::HostMark::resolve(&self.config.sidebar_style, &endpoint.label, false, theme)
                    .apply(row, &format!("{prefix}host-{id}"), &look)
            })
            .child(if carried {
                look.lifted(&format!("host-{id}"), false, theme)
            } else {
                div()
                    .absolute()
                    .left(px(look.inset()))
                    .right(px(look.inset()))
                    .top_0()
                    .bottom_0()
                    .rounded(px(look.density.radius))
                    .group_hover(HOST_GROUP, move |style| style.bg(hover))
            })
            .text_color(rgb(text))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_host_menu(&menu_id, event.position, window, cx);
                    this.menu.opening_right_click = this.menu.page == Some(crate::menu::Page::Host);
                }),
            )
            .when(self.config.show_device_icons, |row| {
                row.child(
                    div()
                        .debug_selector(|| format!("{prefix}host-origin-{id}"))
                        .flex_none()
                        .child(icon(
                            origin.icon(),
                            ORIGIN,
                            if endpoint.enabled {
                                theme.subtext()
                            } else {
                                theme.muted
                            },
                        )),
                )
            })
            .when(removing, |row| {
                row.child(removing_dot("host-removing", theme))
            })
            .child(
                div()
                    .debug_selector(|| format!("{prefix}host-label-{id}"))
                    .flex_shrink_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(scale.name))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(label_text(&endpoint.label)),
            )
            .child(
                div()
                    .debug_selector(|| format!("{prefix}host-kind-{id}"))
                    .flex_shrink(100.)
                    .min_w_0()
                    .truncate()
                    .text_size(px(scale.secondary))
                    .text_color(rgb(theme.muted))
                    .child(label_text(&kind(origin))),
            )
            .child(
                div()
                    .debug_selector(|| format!("{prefix}host-dot-{id}"))
                    .size(px(DOT))
                    .flex_none()
                    .rounded_full()
                    .bg(rgb(host_dot(&live.status, theme))),
            )
            .child(
                div()
                    .ml_auto()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(2.))
                    .text_color(rgb(theme.muted))
                    .children(reading.and_then(|reading| {
                        load(format!("{prefix}host-load-{id}"), reading, scale.detail)
                    }))
                    .child(fold)
                    .child(new_space),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.select_endpoint(&select_id, cx);
                window.focus(&this.focus, cx);
            }))
    }
}
