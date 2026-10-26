//! The panel for the uncommitted changes: the branch and its upstream, then
//! each section's files, as hgg shows the working tree. Read-only: changes
//! are staged, discarded or committed from a terminal.

use super::{GitGraphView, detail::Panel};
use crate::{
    design::NoteTone,
    git_graph::worktree::{Section, WorktreeStatus, upstream_label},
};

/// The panel's padding and its captions' column, as the commit's detail
/// draws them.
const PADDING: f32 = 14.;
const TERM_WIDTH: f32 = 76.;
use gpui::{prelude::*, *};

impl GitGraphView {
    pub(super) fn render_worktree(
        &self,
        panel: &Panel,
        status: &WorktreeStatus,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let view = self.look.view();
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
                        .w(view.at(TERM_WIDTH))
                        .text_size(px(view.scale().secondary))
                        .text_color(view.muted())
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
            .px(view.at(PADDING))
            .pb(view.at(12.))
            .flex()
            .flex_col()
            .gap_2()
            .overflow_y_scroll()
            .when(panel.diff_shown(), |el| el.max_h(relative(0.45)))
            .when(!panel.diff_shown(), |el| el.flex_1())
            .child(row("Branch", branch, "graph-branch-name"))
            .child(row("Upstream", upstream_label(status), "graph-upstream"))
            .child(view.dialog().note(
                "graph-worktree-readonly".to_owned(),
                NoteTone::Info,
                "Read-only: stage, discard or commit from a terminal.",
            ));
        if status.files.is_empty() {
            info = info.child(
                div()
                    .text_color(view.muted())
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
            info = info
                .child(
                    view.menu()
                        .section(
                            format!("graph-section-{key}"),
                            format!("{} \u{00b7} {}", section.title(), files.len()),
                        )
                        .px_0()
                        .when(section == Section::Conflicted, |heading| {
                            heading.text_color(view.settings().danger())
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
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
