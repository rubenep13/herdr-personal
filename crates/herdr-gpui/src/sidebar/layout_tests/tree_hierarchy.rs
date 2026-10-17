use super::tree_layout::{draw, tree_window};
use super::*;
use crate::config::TreeHierarchy;

/// The two-host tree drawn with `hierarchy`.
fn drawn(
    cx: &mut gpui::TestAppContext,
    hierarchy: TreeHierarchy,
) -> (Entity<HerdrWindow>, &mut gpui::VisualTestContext) {
    let (view, cx) = tree_window(cx, true);
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            view.config.sidebar_style.hierarchy = hierarchy;
            cx.notify();
        })
    });
    draw(cx);
    (view, cx)
}

fn bounds(cx: &mut gpui::VisualTestContext, selector: String) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(selector.clone().leak())
}

/// Each row of the local host below its header, by the key its decoration
/// carries and the selector of the row itself.
const LOCAL_ROWS: [(&str, &str); 6] = [
    ("local-w0", "workspace-local-w0"),
    ("local-p0", "agent-local-p0"),
    ("local-t1", "tab-row-local-t1"),
    ("local-w1", "workspace-local-w1"),
    ("local-p1", "agent-local-p1"),
    ("local-w2", "workspace-local-w2"),
];

#[gpui::test]
fn indent_steps_each_level_in(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = drawn(cx, TreeHierarchy::Indent);
    let host = cx.debug_bounds("host-origin-local").unwrap();
    let space = cx.debug_bounds("space-icon-w0").unwrap();
    let agent = cx.debug_bounds("agent-tile-p0").unwrap();
    assert!(space.left() > host.left() + px(16.), "{space:?} {host:?}");
    assert!(agent.left() > space.left() + px(16.), "{agent:?} {space:?}");
    for (key, _) in LOCAL_ROWS {
        assert!(bounds(cx, format!("tree-guide-{key}")).is_none(), "{key}");
        assert!(bounds(cx, format!("tree-block-{key}")).is_none(), "{key}");
    }
}

#[gpui::test]
fn guide_runs_down_every_child_of_a_host(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = drawn(cx, TreeHierarchy::Guide);
    let icon = cx.debug_bounds("host-origin-local").unwrap();
    assert!(cx.debug_bounds("tree-guide-local").is_none());
    let mut previous: Option<Bounds<Pixels>> = None;
    for (key, row) in LOCAL_ROWS {
        let guide = bounds(cx, format!("tree-guide-{key}")).unwrap_or_else(|| panic!("{key}"));
        let row = bounds(cx, row.into()).unwrap();
        // The line drops from the host's mark, left of the row's own content.
        assert!(
            guide.left() >= icon.left() && guide.right() <= icon.right(),
            "{key}"
        );
        assert!(
            guide.right() <= row.left() + px(16.),
            "{key}: {guide:?} {row:?}"
        );
        assert_eq!(
            (guide.top(), guide.bottom()),
            (row.top(), row.bottom()),
            "{key}"
        );
        // One unbroken line: each segment starts where the last one ended.
        if let Some(previous) = previous {
            assert_eq!(guide.top(), previous.bottom(), "{key}");
        }
        previous = Some(guide);
    }
}

#[gpui::test]
fn block_wraps_each_host_and_rounds_its_ends(cx: &mut gpui::TestAppContext) {
    let (_view, cx) = drawn(cx, TreeHierarchy::Block);
    let header = cx.debug_bounds("tree-block-local").unwrap();
    let mut previous = header;
    for (key, _) in LOCAL_ROWS {
        let block = bounds(cx, format!("tree-block-{key}")).unwrap_or_else(|| panic!("{key}"));
        assert!(bounds(cx, format!("tree-guide-{key}")).is_none(), "{key}");
        // The host's rows paint one block, the header's width, edge to edge.
        assert_eq!(
            (block.left(), block.right()),
            (header.left(), header.right())
        );
        assert_eq!(block.top(), previous.bottom(), "{key}");
        previous = block;
    }
    // The block keeps clear of the sidebar's edges, and the next host's
    // block starts after a gap.
    let sidebar = cx.debug_bounds("sidebar").unwrap();
    assert!(header.left() > sidebar.left());
    let next = cx.debug_bounds("tree-block-ssh:test").unwrap();
    assert!(
        next.top() >= previous.bottom() + px(10.),
        "{next:?} {previous:?}"
    );
}

#[gpui::test]
fn hierarchy_is_computed_per_host_with_repeated_ids(cx: &mut gpui::TestAppContext) {
    let (view, cx) = drawn(cx, TreeHierarchy::Block);
    // Both hosts list the same IDs; each host's last row is its own w2.
    let lasts = cx.update(|_, cx| {
        let view = view.read(cx);
        let tree = view.tree_layout("", cx);
        [0, 1].map(|host| tree.last(host).cloned())
    });
    for (last, endpoint) in lasts.iter().zip(["local", "ssh:test"]) {
        match last {
            Some(crate::sidebar::tree::NodeId::Space { context, workspace }) => {
                assert_eq!(workspace, "w2");
                assert_eq!(context.endpoint.endpoint_id, endpoint);
            }
            other => panic!("{endpoint}: {other:?}"),
        }
    }
    // Each host's gap follows its own last row, not the other host's.
    let local_last = cx.debug_bounds("tree-block-local-w2").unwrap();
    let remote_header = cx.debug_bounds("tree-block-ssh:test").unwrap();
    assert!(remote_header.top() >= local_last.bottom() + px(10.));
    let remote_first = cx.debug_bounds("tree-block-ssh:test-w0").unwrap();
    assert_eq!(remote_first.top(), remote_header.bottom());
}

#[gpui::test]
fn child_count_is_unchanged(cx: &mut gpui::TestAppContext) {
    for hierarchy in [
        TreeHierarchy::Indent,
        TreeHierarchy::Block,
        TreeHierarchy::Guide,
    ] {
        let (view, cx) = drawn(cx, hierarchy);
        let (children, rows) = cx.update(|_, cx| {
            let view = view.read(cx);
            let scroll = &view.sidebar_scroll[0];
            let children = (0..)
                .take_while(|&child| scroll.bounds_for_item(child).is_some())
                .count();
            (children, view.tree_rows("", cx).len())
        });
        // One list child per projected row: each host's header and its rows.
        assert_eq!(children, rows, "{hierarchy:?}");
    }
}
