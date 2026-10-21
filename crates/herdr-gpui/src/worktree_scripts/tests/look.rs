//! The script trust question in the redesigned dialog: the script in a well
//! of the terminal's face, and its trust button last on the right.

use super::*;
use crate::worktree_scripts::{Config, Job};
use gpui::{px, size};

#[gpui::test]
fn worktree_script_shows_its_command_in_a_well(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(900.), px(700.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            let config =
                Config::parse(b"[scripts]\nrun = \"make dev\"\nsetup = \"npm ci\"\n").unwrap();
            view.worktree_script = Some(Job {
                launch: launch(view, ScriptKind::Run, None),
                step: Step::Asking {
                    config,
                    shown: None,
                },
                cancel: Default::default(),
            });
            view.open_menu(window, cx);
            view.menu.page = Some(Page::WorktreeScript);
        });
        window.draw(cx).clear(cx);
    });
    let panel = cx.debug_bounds("menu-panel").unwrap();
    let header = cx.debug_bounds("worktree-script-header").unwrap();
    let cross = cx.debug_bounds("worktree-script-dismiss").unwrap();
    assert!(header.contains(&cross.center()), "{cross:?} in {header:?}");
    let script = cx.debug_bounds("worktree-script-run").unwrap();
    assert!(panel.contains(&script.origin) && script.right() <= panel.right());
    let skip = cx.debug_bounds("worktree-script-skip").unwrap();
    let trust = cx.debug_bounds("worktree-script-trust-run").unwrap();
    assert!(trust.left() > skip.right(), "{skip:?} {trust:?}");
    assert_eq!(trust.size.height, px(30.));
}
