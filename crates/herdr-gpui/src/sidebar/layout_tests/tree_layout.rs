use super::*;
use crate::{
    config::LayoutMode,
    controls::Command,
    sidebar::tree::{EndpointKey, NodeId, SnapshotKey, storage::TreeState},
    state::ConnectionStatus,
};

/// Three spaces with the fixture's two agents, in the Tree layout: `p0`
/// under `w0`, `p1` under `w1`, `w2` empty. `remote` adds a second host
/// with the same IDs, so every row names its host in its selector.
pub(super) fn tree_window(
    cx: &mut gpui::TestAppContext,
    remote: bool,
) -> (Entity<HerdrWindow>, &mut gpui::VisualTestContext) {
    let (fixture, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| {
            let mut view = fixture_window(window, cx);
            view.config.layout.mode = LayoutMode::Tree;
            let mut listing = snapshot(3);
            listing.focused_workspace_id = Some("w0".into());
            view.live.snapshot = Some(Arc::new(listing));
            view.live.status = ConnectionStatus::Connected;
            view.endpoints[0].live = view.live.clone();
            if remote {
                let mut host = crate::endpoint::Endpoint::new(
                    "ssh:test".into(),
                    "Build box".into(),
                    ConnectTarget::Ssh {
                        target: "unused".into(),
                        session: "default".into(),
                    },
                    true,
                );
                let mut listing = snapshot(3);
                listing.agents[0].agent_status = AgentStatus::Blocked;
                host.live.snapshot = Some(Arc::new(listing));
                host.live.status = ConnectionStatus::Connected;
                view.endpoints.push(host);
            }
            view
        });
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        SidebarFixture(view)
    });
    let view = cx.update(|_, cx| fixture.read(cx).0.clone());
    cx.simulate_resize(size(px(800.), px(700.)));
    // The tick gives the layout its search field.
    cx.update(|_, cx| view.update(cx, |view, cx| view.poll_devices_overview(cx)));
    cx.run_until_parked();
    draw(cx);
    (view, cx)
}

pub(super) fn draw(cx: &mut gpui::VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
}

fn top(cx: &mut gpui::VisualTestContext, selector: &'static str) -> Pixels {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("missing {selector}"))
        .top()
}

/// Asserts the selectors are painted top to bottom in this order.
fn painted_in_order(cx: &mut gpui::VisualTestContext, selectors: &[&'static str]) {
    let tops: Vec<_> = selectors.iter().map(|selector| top(cx, selector)).collect();
    for (pair, selector) in tops.windows(2).zip(&selectors[1..]) {
        assert!(
            pair[0] < pair[1],
            "{selector} is not below its predecessor: {tops:?}"
        );
    }
}

/// The first host's boot, keyed as the window keys it.
fn local_context(view: &Entity<HerdrWindow>, cx: &mut gpui::VisualTestContext) -> SnapshotKey {
    view.read_with(cx, |view, _| SnapshotKey {
        endpoint: EndpointKey::new(&view.endpoints[0].id, &view.endpoints[0].connection.target),
        boot_id: "layout-test".into(),
    })
}

fn space(context: &SnapshotKey, workspace: &str) -> NodeId {
    NodeId::Space {
        context: context.clone(),
        workspace: workspace.into(),
    }
}

fn steps(
    view: &Entity<HerdrWindow>,
    cx: &mut gpui::VisualTestContext,
    agents: bool,
) -> Vec<String> {
    cx.update(|_, cx| {
        let view = view.read(cx);
        let rows = if agents {
            view.sidebar_agents(cx)
        } else {
            view.sidebar_workspaces(cx)
        };
        rows.into_iter()
            .map(|(host, id)| format!("{host}:{id}"))
            .collect()
    })
}

#[gpui::test]
fn agents_nest_under_their_space_under_their_host(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = tree_window(cx, true);
    assert!(cx.debug_bounds("header-tree").is_some());
    assert!(cx.debug_bounds("tree-search").is_some());
    // The agents sit in the tree, so the separate panel is gone.
    assert!(cx.debug_bounds("header-agents").is_none());
    assert!(cx.debug_bounds("agents-section").is_none());
    painted_in_order(
        cx,
        &[
            "host-local",
            "workspace-local-w0",
            "agent-local-p0",
            "workspace-local-w1",
            "agent-local-p1",
            "workspace-local-w2",
            "host-ssh:test",
            "workspace-ssh:test-w0",
            "agent-ssh:test-p0",
            "workspace-ssh:test-w1",
            "agent-ssh:test-p1",
        ],
    );
}

#[gpui::test]
fn an_agent_steps_in_from_its_space_and_names_only_itself(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = tree_window(cx, false);
    let space = cx.debug_bounds("name-herdr").unwrap();
    let agent = cx.debug_bounds("name-agent-p0").unwrap();
    let detail = cx.debug_bounds("detail-agent-p0").unwrap();
    assert!(agent.left() > space.left(), "{agent:?} {space:?}");
    // The tab names the row; the agent's kind follows on the line under it.
    assert!(agent.bottom() <= detail.top(), "{agent:?} {detail:?}");
}

#[gpui::test]
fn folding_a_space_hides_its_agents_and_counts_their_attention(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    let context = local_context(&view, cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let mut listing = (**view.live.snapshot.as_ref().unwrap()).clone();
            listing.agents[0].agent_status = AgentStatus::Blocked;
            view.live.snapshot = Some(Arc::new(listing));
            cx.notify();
        });
        TreeState::update(cx, |tree| {
            (tree.set_space_collapsed(&context, "w0", true), ())
        });
    });
    draw(cx);
    assert!(cx.debug_bounds("row-agent-p0").is_none());
    assert!(cx.debug_bounds("row-agent-p1").is_some());
    // The folded space shows how many agents want attention, in its own row.
    assert!(cx.debug_bounds("attention-w0").is_some());
    assert_eq!(steps(&view, cx, true), ["0:p1"]);
    // Unfolding through its control shows the agent again.
    cx.update(|_, cx| {
        TreeState::update(cx, |tree| {
            (tree.set_space_collapsed(&context, "w0", false), ())
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("row-agent-p0").is_some());
}

#[gpui::test]
fn the_search_keeps_each_match_with_its_parents(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, true);
    let input = view.read_with(cx, |view, _| {
        view.devices_overview
            .tree_search
            .as_ref()
            .unwrap()
            .input
            .clone()
    });
    cx.update(|window, cx| window.focus(&input.read(cx).focus.clone(), cx));
    cx.simulate_input("review");
    draw(cx);
    for kept in [
        "host-local",
        "workspace-local-w0",
        "agent-local-p0",
        "host-ssh:test",
        "agent-ssh:test-p0",
    ] {
        assert!(cx.debug_bounds(kept).is_some(), "{kept}");
    }
    for dropped in ["workspace-local-w1", "agent-local-p1", "workspace-local-w2"] {
        assert!(cx.debug_bounds(dropped).is_none(), "{dropped}");
    }
}

#[gpui::test]
fn dropping_a_space_reorders_only_this_clients_tree(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    let context = local_context(&view, cx);
    let (w0, w1, w2) = (
        space(&context, "w0"),
        space(&context, "w1"),
        space(&context, "w2"),
    );
    cx.update(|_, cx| view.update(cx, |view, cx| view.drop_tree_node(&w2, &w0, cx)));
    draw(cx);
    painted_in_order(cx, &["row-another workspace", "row-herdr", "row-agent-p0"]);
    // The shortcuts step through the rows as they are painted.
    assert_eq!(steps(&view, cx, false), ["0:w2", "0:w0", "0:w1"]);
    assert_eq!(steps(&view, cx, true), ["0:p0", "0:p1"]);
    // Dropping on a sibling lower down puts the row after it.
    cx.update(|_, cx| view.update(cx, |view, cx| view.drop_tree_node(&w2, &w1, cx)));
    assert_eq!(steps(&view, cx, false), ["0:w0", "0:w1", "0:w2"]);
    // The daemon's order, which every client shares, is untouched.
    view.read_with(cx, |view, _| {
        let listing = view.live.snapshot.as_ref().unwrap();
        let ids: Vec<_> = listing
            .workspaces
            .iter()
            .map(|w| w.workspace_id.as_str())
            .collect();
        assert_eq!(ids, ["w0", "w1", "w2"]);
        assert!(view.workspace_drag.is_none());
    });
}

#[gpui::test]
fn a_plugin_view_is_named_and_keeps_its_own_agent_order(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, true);
    let context = local_context(&view, cx);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let mut listing = (**view.live.snapshot.as_ref().unwrap()).clone();
            listing.agent_view_label = Some("Blocked first".into());
            listing.agent_order = vec!["p1".into()];
            view.live.snapshot = Some(Arc::new(listing));
            cx.notify();
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("tree-filter-local").is_some());
    assert!(cx.debug_bounds("tree-filter-ssh:test").is_none());
    assert!(cx.debug_bounds("agent-local-p0").is_none());
    assert!(cx.debug_bounds("agent-local-p1").is_some());
    let agent = |pane: &str| NodeId::Agent {
        context: context.clone(),
        workspace: "w1".into(),
        pane: pane.into(),
    };
    let (p0, p1) = (agent("p0"), agent("p1"));
    cx.update(|_, cx| view.update(cx, |view, cx| view.drop_tree_node(&p1, &p0, cx)));
    cx.update(|_, cx| {
        assert!(TreeState::of(cx).is_none_or(|state| state.preferences().agent_orders.is_empty()));
    });
}

#[gpui::test]
fn move_commands_step_the_focused_space_among_its_siblings(cx: &mut gpui::TestAppContext) {
    let (view, cx) = tree_window(cx, false);
    // The fixture focuses w0, the first space.
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.command(Command::MoveSpaceUp, window, cx);
        })
    });
    assert_eq!(steps(&view, cx, false), ["0:w0", "0:w1", "0:w2"]);
    for _ in 0..2 {
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.command(Command::MoveSpaceDown, window, cx)
            })
        });
    }
    assert_eq!(steps(&view, cx, false), ["0:w1", "0:w2", "0:w0"]);
    draw(cx);
    painted_in_order(cx, &["row-another workspace", "row-herdr", "row-agent-p0"]);
}
