// Named imports: the parent's glob brings in GPUI's `test`, which would
// shadow the one `#[gpui::test]` expands to.
use super::{Cancel, Running, Target};
use crate::{
    HerdrWindow,
    agent_actions::{DEADLINE, Error, StartOutcome},
    sidebar::{layout_tests::fixture_window, tree::EndpointKey},
    teleport::AgentKind,
    window::Flash,
};
use std::sync::atomic::Ordering;

/// The fixture's first space, as a start made now would target it.
fn target(view: &HerdrWindow) -> Target {
    let endpoint = &view.endpoints[0];
    Target {
        endpoint_id: endpoint.id.clone(),
        key: EndpointKey::new(&endpoint.id, &endpoint.connection.target),
        connect: endpoint.connection.target.clone(),
        generation: endpoint.generation,
        boot_id: view.live.snapshot.as_ref().unwrap().boot_id.clone(),
        workspace_id: "w0".into(),
        label: "herdr".into(),
    }
}

/// Finishes a start in flight with `outcome`, after `change` alters the
/// window as a late answer might find it, and returns whether the flash is a
/// success, its text, and whether the start's worker was told to stop.
fn finish(
    cx: &mut gpui::TestAppContext,
    change: impl FnOnce(&mut HerdrWindow, &mut Target),
    outcome: StartOutcome,
) -> (bool, String, bool) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        let mut target = target(view);
        let cancel = Cancel::default();
        let stopped = cancel.0.clone();
        view.agent_start.running = Some(Running {
            kind: AgentKind::Claude,
            _cancel: cancel,
        });
        change(view, &mut target);
        view.finish_agent_start(&target, outcome, cx);
        // Nothing is left in flight, and nothing starts again on its own.
        assert!(view.agent_start.running.is_none());
        assert!(view.agent_start.choosing.is_none());
        let (flash, _) = view.flash.clone().unwrap();
        (
            flash.tone == Flash::success("").tone,
            flash.text.to_string(),
            stopped.load(Ordering::Acquire),
        )
    })
}

fn ready() -> StartOutcome {
    StartOutcome::Ready {
        pane_id: "w0:p9".into(),
    }
}

#[gpui::test]
fn a_ready_agent_on_the_same_boot_is_reported_ready(cx: &mut gpui::TestAppContext) {
    let (success, text, stopped) = finish(cx, |_, _| {}, ready());
    assert!(success, "{text}");
    assert!(text.contains("claude is ready in herdr"), "{text}");
    assert!(stopped);
}

#[gpui::test]
fn a_late_answer_after_a_daemon_restart_focuses_nothing(cx: &mut gpui::TestAppContext) {
    let (success, text, _) = finish(
        cx,
        |_, target| target.boot_id = "an earlier boot".into(),
        ready(),
    );
    assert!(!success, "{text}");
    assert!(text.contains("host changed since"), "{text}");
}

#[gpui::test]
fn a_late_answer_after_a_reconnect_or_another_session_focuses_nothing(
    cx: &mut gpui::TestAppContext,
) {
    for change in [
        (|_: &mut HerdrWindow, target: &mut Target| target.generation += 1)
            as fn(&mut HerdrWindow, &mut Target),
        |_, target| target.key.session = "remote:other".into(),
        |_, target| target.endpoint_id = "ssh:gone".into(),
    ] {
        let (success, text, _) = finish(cx, change, ready());
        assert!(!success, "{text}");
        assert!(text.contains("was not focused"), "{text}");
    }
}

#[gpui::test]
fn failed_and_unknown_starts_say_so_and_are_not_retried(cx: &mut gpui::TestAppContext) {
    let (success, text, _) = finish(
        cx,
        |_, _| {},
        StartOutcome::Failed(Error::Rejected {
            code: "agent_pane_busy".into(),
            message: "not an available shell".into(),
        }),
    );
    assert!(!success, "{text}");
    assert!(text.contains("was not started"), "{text}");
    assert!(text.contains("agent_pane_busy"), "{text}");
    let (success, text, _) = finish(
        cx,
        |_, _| {},
        StartOutcome::Unknown {
            pane_id: "w0:p9".into(),
            error: Error::Deadline(DEADLINE),
        },
    );
    assert!(!success, "{text}");
    assert!(text.contains("Could not confirm"), "{text}");
    assert!(text.contains("not retried"), "{text}");
}

#[gpui::test]
fn the_menu_offers_a_start_only_with_a_space_and_none_in_flight(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        // No workspace menu is open.
        assert!(!view.start_agent_available());
        view.agent_start.running = Some(Running {
            kind: AgentKind::Codex,
            _cancel: Cancel::default(),
        });
        assert!(!view.start_agent_available());
    });
}
