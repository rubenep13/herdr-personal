//! The graph's keys, which act only with the tab focused and no modifier,
//! so they never take the app's own shortcuts: j/↓ and k/↑ move the
//! selection, h selects HEAD, c copies the selected commit's hash and r
//! reads the repository again.

use super::GitGraphView;
use crate::git_graph::model::CommitKind;
use gpui::{prelude::*, *};

impl GitGraphView {
    pub(super) fn on_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let modifiers = &event.keystroke.modifiers;
        if modifiers.platform || modifiers.control || modifiers.alt || modifiers.function {
            return;
        }
        let handled = match (event.keystroke.key.as_str(), modifiers.shift) {
            ("j" | "down", false) => self.step(1, cx),
            ("k" | "up", false) => self.step(-1, cx),
            ("h", false) => self.select_head(cx),
            ("c", false) => self.copy_hash(cx),
            ("r", false) => {
                self.reload(cx);
                true
            }
            ("enter", false) => {
                let row = self.selected_row().unwrap_or_default();
                self.open_detail(row, cx);
                true
            }
            ("escape", _) if self.close_detail(cx) => true,
            ("escape", _) if self.column_menu => {
                self.column_menu = false;
                cx.notify();
                true
            }
            _ => false,
        };
        if handled {
            cx.stop_propagation();
        }
    }

    /// Moves the selection `delta` rows, within the rows loaded; the first
    /// move from no selection selects the top row.
    fn step(&mut self, delta: isize, cx: &mut Context<Self>) -> bool {
        let Some(last) = self
            .history
            .as_ref()
            .and_then(|history| history.commits.len().checked_sub(1))
        else {
            return false;
        };
        let row = match self.selected_row() {
            Some(row) => row.saturating_add_signed(delta).min(last),
            None => 0,
        };
        // An open panel follows the selection.
        if self.panel.is_some() {
            self.open_detail(row, cx);
        } else {
            self.select_row(row, cx);
        }
        true
    }

    fn select_head(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(row) = self.history.as_ref().and_then(|history| {
            history
                .head
                .as_deref()
                .and_then(|head| history.row_of(head))
        }) else {
            return false;
        };
        self.select_row(row, cx);
        true
    }

    /// Copies the selected commit's full hash; the uncommitted changes have
    /// none.
    fn copy_hash(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(commit) = self.selected_row().and_then(|row| {
            self.history
                .as_ref()
                .and_then(|history| history.commits.get(row))
        }) else {
            return false;
        };
        if commit.kind == CommitKind::Worktree {
            return false;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(commit.oid.clone()));
        true
    }
}
