//! Settings > Sidebar > Tree: the tree layout's four options, drafted like the
//! rest of the page, and switched off while another layout is chosen.
use super::*;
use crate::config::preferences::Preference;
use crate::config::{LayoutMode, TreeHierarchy};

type Edits = Arc<Mutex<Vec<Preference>>>;

fn sidebar_page(
    cx: &mut TestAppContext,
    mode: LayoutMode,
) -> (Entity<SettingsWindow>, &mut VisualTestContext, Edits) {
    let source = cx.add_window(crate::sidebar::layout_tests::fixture_window);
    let edits = Arc::new(Mutex::new(Vec::new()));
    let captured = edits.clone();
    let weak = cx.update(|cx| source.update(cx, |_, _, cx| cx.weak_entity()).unwrap());
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let mut view = SettingsWindow::new(weak, cx);
        view.controls.preference_io = Some(controls::preferences::PreferenceIo::recording(
            move |edit| captured.lock().unwrap().push(edit),
        ));
        view.initialize_controls(cx);
        for config in [&mut view.config, &mut view.loaded_config] {
            config.layout.mode = mode;
        }
        view.section = Section::Sidebar;
        window.focus(&view.focus, cx);
        view
    });
    cx.simulate_resize(size(px(1200.), px(2400.)));
    draw(cx);
    (view, cx, edits)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let at = cx.debug_bounds(selector).unwrap().center();
    cx.simulate_click(at, Modifiers::default());
    cx.run_until_parked();
    draw(cx);
}

fn hierarchy(view: &Entity<SettingsWindow>, cx: &mut VisualTestContext) -> TreeHierarchy {
    view.read_with(cx, |view, _| view.config.sidebar_style.hierarchy)
}

#[gpui::test]
fn tree_options_draft_like_the_rest_of_the_sidebar_page(cx: &mut TestAppContext) {
    let (view, cx, edits) = sidebar_page(cx, LayoutMode::Tree);
    for row in [
        "sidebar-hierarchy",
        "sidebar-tree-tabs",
        "sidebar-agent-details",
        "sidebar-state-text",
    ] {
        let row: &'static str = row;
        assert!(cx.debug_bounds(row).is_some(), "{row}");
    }
    click(cx, "sidebar-hierarchy-control-2");
    assert_eq!(hierarchy(&view, cx), TreeHierarchy::Block);
    view.read_with(cx, |view, _| {
        assert!(view.sidebar_dirty());
        assert!(
            view.footer_text().contains("sidebar"),
            "{}",
            view.footer_text()
        );
    });
    assert!(
        edits.lock().unwrap().is_empty(),
        "nothing is written before a save"
    );
    view.update(cx, |view, cx| view.save_pending_drafts(cx));
    cx.run_until_parked();
    assert_eq!(
        *edits.lock().unwrap(),
        vec![Preference::SidebarHierarchy(TreeHierarchy::Block)]
    );
}

#[gpui::test]
fn tree_options_switch_off_outside_the_tree_layout(cx: &mut TestAppContext) {
    let (view, cx, _) = sidebar_page(cx, LayoutMode::Tree);
    assert!(cx.debug_bounds("sidebar-hierarchy-reason").is_none());
    view.update(cx, |view, cx| {
        view.accept_layout_choice(LayoutMode::default(), cx)
    });
    draw(cx);
    assert!(cx.debug_bounds("sidebar-hierarchy-reason").is_some());
    click(cx, "sidebar-hierarchy-control-1");
    assert_eq!(
        hierarchy(&view, cx),
        TreeHierarchy::Guide,
        "a disabled row ignores clicks"
    );
    view.update(cx, |view, cx| {
        view.accept_layout_choice(LayoutMode::Tree, cx)
    });
    draw(cx);
    assert!(cx.debug_bounds("sidebar-hierarchy-reason").is_none());
    click(cx, "sidebar-hierarchy-control-1");
    assert_eq!(hierarchy(&view, cx), TreeHierarchy::Indent);
}

#[test]
fn search_finds_the_tree_options() {
    let ids = |query: &str| {
        catalog::search(query)
            .into_iter()
            .map(|index| catalog::CATALOG[index].id)
            .collect::<Vec<_>>()
    };
    assert!(ids("hierarchy").contains(&"sidebar-hierarchy"));
    assert!(ids("state").contains(&"sidebar-state-text"));
    assert!(ids("agent details").contains(&"sidebar-agent-details"));
}
