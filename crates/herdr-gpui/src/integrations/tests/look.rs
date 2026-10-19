//! The Integrations list in the redesigned look: one card of rows, each with
//! its state in a chip.
use super::*;
use crate::design::Tone;

#[test]
fn integration_states_map_to_chip_tones() {
    assert_eq!(tone(State::Current), Tone::Good);
    assert_eq!(tone(State::Outdated), Tone::Warn);
    assert_eq!(tone(State::NotInstalled), Tone::Neutral);
}

struct Harness(Entity<HerdrWindow>);

impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            self.0
                .update(cx, |source, cx| source.render_filtered_integrations("", cx)),
        )
    }
}

#[gpui::test]
fn integration_rows_share_one_card(cx: &mut TestAppContext) {
    let source = cx.add_window(crate::sidebar::layout_tests::fixture_window);
    let entity = source
        .update(cx, |window, _, cx| {
            let epoch = window.selection_epoch;
            let endpoint = &mut window.endpoints[window.selected_endpoint];
            endpoint.live.status = crate::state::ConnectionStatus::Connected;
            if endpoint.live.snapshot.is_none() {
                endpoint.live.snapshot = Some(crate::sidebar::layout_tests::snapshot(1).into());
            }
            endpoint.connection.integrations.lock().unwrap().install = true;
            let scope = Scope {
                endpoint: endpoint.id.clone(),
                host: endpoint.label.clone(),
                epoch,
                generation: endpoint.generation,
                boot: endpoint.live.snapshot.as_ref().unwrap().boot_id.clone(),
                inbox: endpoint.connection.integrations.clone(),
            };
            window.integrations.scope = Some(scope);
            window.integrations.rows = parse_list(&list()).unwrap();
            cx.entity()
        })
        .unwrap();
    let (_, cx) = cx.add_window_view(|_, _| Harness(entity));
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let card = cx.debug_bounds("integrations-card").unwrap();
    let install = cx.debug_bounds("install-antigravity_cli").unwrap();
    assert!(
        card.contains(&install.origin) && card.contains(&install.center()),
        "{install:?} in {card:?}"
    );
    assert!(
        cx.debug_bounds("integration-antigravity_cli-state")
            .is_some()
    );
}
