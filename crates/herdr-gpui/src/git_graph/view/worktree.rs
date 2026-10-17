//! The panel for the uncommitted changes: the branch and its upstream, then
//! each section's files, as hgg shows the working tree. Read-only: changes
//! are staged, discarded or committed from a terminal.

use super::{GitGraphView, detail::Panel};
use crate::git_graph::worktree::{Section, WorktreeStatus, upstream_label};
use gpui::{prelude::*, *};

impl GitGraphView {
    pub(super) fn render_worktree(
        &self,
        panel: &Panel,
        status: &WorktreeStatus,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = &self.look.theme;
        let branch = self
            .history
            .as_ref()
            .and_then(|history| {
                let head = history.head.as_ref()?;
                history.chips.get(head)?.iter().find(|chip| chip.current)
            })
            .map_or_else(
                || "HEAD detached".to_owned(),
                |chip| {
                    if chip.label == "HEAD" {
                        "HEAD detached".to_owned()
                    } else {
                        chip.label.clone()
                    }
                },
            );
        let row = |label: &str, value: String, selector: &'static str| {
            div()
                .flex()
                .gap_2()
                .min_w_0()
                .child(
                    div()
                        .flex_none()
                        .w(px(76.))
                        .text_color(rgb(theme.muted))
                        .child(label.to_owned()),
                )
                .child(
                    div()
                        .debug_selector(move || selector.to_owned())
                        .min_w_0()
                        .truncate()
                        .child(value),
                )
        };
        let mut info = div()
            .id("graph-worktree-info")
            .px_4()
            .pb_3()
            .flex()
            .flex_col()
            .gap_2()
            .overflow_y_scroll()
            .when(panel.diff_shown(), |el| el.max_h(relative(0.45)))
            .when(!panel.diff_shown(), |el| el.flex_1())
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(theme.foreground))
                    .child("Uncommitted changes"),
            )
            .child(row("Branch", branch, "graph-branch-name"))
            .child(row("Upstream", upstream_label(status), "graph-upstream"))
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(theme.muted))
                    .child("Read-only: stage, discard or commit from a terminal."),
            );
        if status.files.is_empty() {
            info = info.child(
                div()
                    .text_color(rgb(theme.muted))
                    .child("The working tree is clean."),
            );
        }
        for section in Section::ALL {
            let files: Vec<(usize, &crate::git_graph::detail::FileChange)> = status
                .files
                .iter()
                .enumerate()
                .filter(|(_, (in_section, _))| *in_section == section)
                .map(|(index, (_, file))| (index, file))
                .collect();
            if files.is_empty() {
                continue;
            }
            let key = section.key();
            let heading = if section == Section::Conflicted {
                theme.ink(theme.palette[1])
            } else {
                theme.subtext()
            };
            info = info
                .child(
                    div()
                        .debug_selector(move || format!("graph-section-{key}"))
                        .mt_2()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(heading))
                        .child(format!(
                            "{} \u{00b7} {}",
                            section.title().to_uppercase(),
                            files.len()
                        )),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .rounded(px(4.))
                        .overflow_hidden()
                        .bg(rgb(theme.background))
                        .children(files.into_iter().map(|(index, file)| {
                            self.render_file(index, file, panel.file == Some(index), false, cx)
                        })),
                );
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(info)
            .children(panel.diff().map(|diff| self.render_diff(panel, diff, cx)))
            .into_any_element()
    }
}
