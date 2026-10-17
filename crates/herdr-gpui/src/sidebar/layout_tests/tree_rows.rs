use super::tree_layout::{draw, tree_window};
use super::*;
use crate::{
    config::{AgentDetails, Theme, TreeTabs},
    icons::AgentIcon,
    sidebar::{
        Indicators,
        layouts::{badge_color, logo_color},
    },
};

fn bounds(cx: &mut gpui::VisualTestContext, selector: String) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(selector.leak())
}

fn edit(
    view: &Entity<HerdrWindow>,
    cx: &mut gpui::VisualTestContext,
    change: impl FnOnce(&mut HerdrWindow),
) {
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            change(view);
            cx.notify();
        })
    });
    draw(cx);
}

/// The texts one fresh frame paints.
fn drawn_texts(cx: &mut gpui::VisualTestContext) -> Vec<String> {
    cx.update(|window, cx| {
        cx.default_global::<TextProbes>().0.clear();
        full_draw(window, cx).clear(cx);
        cx.global::<TextProbes>().0.keys().cloned().collect()
    })
}

/// Edits the selected host's snapshot.
fn edit_listing(
    view: &Entity<HerdrWindow>,
    cx: &mut gpui::VisualTestContext,
    change: impl FnOnce(&mut ClientShellSnapshot),
) {
    edit(view, cx, |view| {
        let mut listing = (**view.live.snapshot.as_ref().unwrap()).clone();
        change(&mut listing);
        view.live.snapshot = Some(Arc::new(listing));
        view.endpoints[0].live = view.live.clone();
    });
}

#[gpui::test]
fn tree_rows_show_state_text_only_when_enabled(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    assert!(cx.debug_bounds("agent-badge-p0").is_some());
    assert!(cx.debug_bounds("agent-state-p0").is_none());
    let before = drawn_texts(cx);
    edit(&view, cx, |view| {
        view.config.sidebar_style.state_text = true
    });
    let state = cx.debug_bounds("agent-state-p0").unwrap();
    let kind = cx.debug_bounds("detail-agent-p0").unwrap();
    // The state sits beside the kind on the second line, after it.
    assert!(state.left() > kind.right(), "{state:?} {kind:?}");
    assert!(state.top() >= kind.top() - px(2.) && state.bottom() <= kind.bottom() + px(2.));
    let after = drawn_texts(cx);
    assert_eq!(
        after.iter().filter(|text| *text == "working").count(),
        1,
        "{after:?}"
    );
    assert!(!before.iter().any(|text| text == "working"), "{before:?}");
}

#[gpui::test]
fn tree_rows_show_details_only_with_herdr_details(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    let plain = cx.debug_bounds("row-agent-p0").unwrap();
    edit_listing(&view, cx, |listing| {
        listing.agents[0].tokens = vec![
            ("model".into(), "opus".into()),
            ("summary".into(), "fixing the sidebar".into()),
        ];
    });
    edit(&view, cx, |view| {
        view.config.sidebar_layout.agents =
            toml::from_str(r#"rows = [["state_icon", "agent"], ["$model", "$summary"]]"#).unwrap();
    });
    let details = cx.debug_bounds("agent-details-p0").unwrap();
    let grown = cx.debug_bounds("row-agent-p0").unwrap();
    assert!(grown.size.height > plain.size.height, "{grown:?} {plain:?}");
    assert!(details.bottom() <= grown.bottom());
    let texts = drawn_texts(cx);
    for text in ["opus", "fixing the sidebar"] {
        assert!(texts.iter().any(|drawn| drawn == text), "{text}: {texts:?}");
    }
    edit(&view, cx, |view| {
        view.config.sidebar_style.agent_details = AgentDetails::Off;
    });
    assert!(cx.debug_bounds("agent-details-p0").is_none());
    assert_eq!(cx.debug_bounds("row-agent-p0").unwrap().size, plain.size);
}

#[gpui::test]
fn tree_tab_rows_follow_tree_tabs(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    // The fixture's second tab in w0 runs no agent.
    let tab = cx.debug_bounds("tab-row-local-t1").unwrap();
    let agent = cx.debug_bounds("row-agent-p0").unwrap();
    assert!(tab.top() >= agent.bottom(), "{tab:?} {agent:?}");
    let texts = drawn_texts(cx);
    for text in ["tab 2", "Terminal"] {
        assert!(texts.iter().any(|drawn| drawn == text), "{text}: {texts:?}");
    }
    assert!(cx.debug_bounds("agent-badge-t1").is_none());
    edit(&view, cx, |view| {
        view.config.sidebar_style.tree_tabs = TreeTabs::Agents
    });
    assert!(cx.debug_bounds("tab-row-local-t1").is_none());
}

#[gpui::test]
fn long_names_never_lose_to_the_branch(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    let name = "a-project-name-thirty-chars-xx";
    let branch = "feature/a-branch-name-that-runs-on-for-sixty-characters-long";
    assert_eq!((name.len(), branch.len()), (30, 60));
    edit_listing(&view, cx, |listing| {
        listing.workspaces[0].label = name.into();
        listing.workspaces[0].branch = Some(branch.into());
        listing.tabs[0].label = "a tab label long enough to need cutting short".into();
    });
    edit(&view, cx, |view| view.sidebar_width = Some(220.));
    let row = bounds(cx, format!("row-{name}")).unwrap();
    let name_box = bounds(cx, format!("name-{name}")).unwrap();
    let branch_box = cx.debug_bounds("space-branch-w0").unwrap();
    assert!(
        name_box.size.width > branch_box.size.width,
        "{name_box:?} {branch_box:?}"
    );
    assert!(name_box.right() <= branch_box.left());
    for selector in [
        format!("name-{name}"),
        "space-branch-w0".into(),
        "space-fold-w0".into(),
        "space-more-w0".into(),
    ] {
        let part = bounds(cx, selector.clone()).unwrap();
        assert!(part.right() <= row.right(), "{selector} {part:?} {row:?}");
    }
    let agent = cx.debug_bounds("row-agent-p0").unwrap();
    let badge = cx.debug_bounds("agent-badge-p0").unwrap();
    let title = cx.debug_bounds("name-agent-p0").unwrap();
    assert!(badge.right() <= agent.right(), "{badge:?} {agent:?}");
    assert!(title.right() <= badge.left(), "{title:?} {badge:?}");
}

#[test]
fn light_theme_inks_status_colors() {
    let latte = Theme::builtin("Catppuccin Latte").unwrap();
    let indicators = Indicators::new(None, None, true, &latte);
    for status in [
        AgentStatus::Idle,
        AgentStatus::Working,
        AgentStatus::Done,
        AgentStatus::Blocked,
    ] {
        let color = badge_color(status, indicators, &latte);
        assert_eq!(color, latte.ink(indicators.color(status)));
        // It reads on the agent's tile, which is the theme's active colour.
        let ratio = crate::contrast::ratio(color, latte.active);
        assert!(
            ratio >= latte.contrast.mark_ratio() - 0.05,
            "{status:?}: {ratio}"
        );
    }
    // Claude's orange keeps its hue but is inked to read on a light tile.
    let claude = logo_color(AgentIcon::Claude, &latte);
    assert_eq!(claude, latte.ink(0xd97757));
    let ratio = crate::contrast::ratio(claude, latte.active);
    assert!(
        ratio >= latte.contrast.mark_ratio() - 0.05,
        "claude: {ratio}"
    );
    // A mark without a brand draws in the theme's text colour.
    assert_eq!(
        logo_color(AgentIcon::Codex, &latte),
        latte.ink(latte.foreground)
    );
}

#[gpui::test]
fn tree_space_rows_carry_ports_inside_the_row(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, true);
    let before = cx.debug_bounds("workspace-local-w0").unwrap();
    edit(&view, cx, |view| {
        view.listening_ports.seed(
            crate::usage::Host::Local,
            crate::listening_ports::parse("L 1 *:3000 node\nE 1 w0\n").unwrap(),
        );
    });
    let row = cx.debug_bounds("workspace-local-w0").unwrap();
    let ports = cx.debug_bounds("ports-local-w0").unwrap();
    assert_eq!(row.size, before.size);
    assert!(
        ports.top() >= row.top() && ports.bottom() <= row.bottom(),
        "{ports:?} {row:?}"
    );
    assert!(ports.right() <= row.right());
    assert!(cx.debug_bounds("port-local-w0-3000").is_some());
}

#[gpui::test]
fn folded_space_counts_attention_in_a_pill(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    assert!(cx.debug_bounds("attention-w0").is_none());
    edit_listing(&view, cx, |listing| {
        listing.agents[0].agent_status = AgentStatus::Blocked;
    });
    let context = view.read_with(cx, |view, _| crate::sidebar::tree::SnapshotKey {
        endpoint: crate::sidebar::tree::EndpointKey::new(
            &view.endpoints[0].id,
            &view.endpoints[0].connection.target,
        ),
        boot_id: "layout-test".into(),
    });
    cx.update(|_, cx| {
        crate::sidebar::tree::storage::TreeState::update(cx, |tree| {
            (tree.set_space_collapsed(&context, "w0", true), ())
        })
    });
    draw(cx);
    let pill = cx.debug_bounds("attention-w0").unwrap();
    let row = cx.debug_bounds("row-herdr").unwrap();
    assert!(pill.right() <= row.right() && pill.left() > row.left());
    // The count no longer rides in the name.
    let texts = drawn_texts(cx);
    assert!(
        !texts.iter().any(|text| text == "herdr \u{00b7} 1"),
        "{texts:?}"
    );
}
