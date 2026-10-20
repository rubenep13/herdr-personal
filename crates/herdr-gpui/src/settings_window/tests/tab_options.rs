//! Settings > Tabs & panes: where the tab strips sit and what each tab shows,
//! and Herdr's own position, which only applies while Position follows it.
use super::*;
use crate::config::preferences::Preference;
use crate::config::{TabContent, TabPlacement};

type Edits = Arc<Mutex<Vec<Preference>>>;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

fn tabs_page(
    cx: &mut TestAppContext,
    placement: TabPlacement,
) -> (Entity<SettingsWindow>, &mut VisualTestContext, Edits) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    let edits = Arc::new(Mutex::new(Vec::new()));
    let captured = edits.clone();
    cx.simulate_resize(size(px(1200.), px(2200.)));
    view.update(cx, |view, cx| {
        view.controls.preference_io = Some(controls::preferences::PreferenceIo::recording(
            move |edit| captured.lock().unwrap().push(edit),
        ));
        view.shared = Some(
            herdr_settings::Settings::parse_text("[ui]\ntab_bar_position = 'bottom'").unwrap(),
        );
        view.config.tabs_style.placement = placement;
        view.section = Section::Tabs;
        cx.notify();
    });
    draw(cx);
    (view, cx, edits)
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let at = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector}"))
        .center();
    cx.simulate_click(at, Default::default());
}

#[gpui::test]
fn tab_placement_and_content_save_as_local_preferences(cx: &mut TestAppContext) {
    let (_, cx, edits) = tabs_page(cx, TabPlacement::Subheader);
    click(cx, "tabs-placement-control-2");
    cx.run_until_parked();
    assert_eq!(
        edits.lock().unwrap().last(),
        Some(&Preference::TabsPosition(TabPlacement::Herdr))
    );
    draw(cx);
    click(cx, "tabs-content-control-1");
    cx.run_until_parked();
    assert_eq!(
        edits.lock().unwrap().last(),
        Some(&Preference::TabsContent(TabContent::Agent))
    );
}

#[gpui::test]
fn herdrs_position_only_applies_when_following_herdr(cx: &mut TestAppContext) {
    // Elsewhere Herdr's file is read-only, so the row never enables.
    if !cfg!(unix) {
        return;
    }
    let (view, cx, _) = tabs_page(cx, TabPlacement::Subheader);
    let shared = Arc::new(Mutex::new(Vec::new()));
    view.update(cx, |view, _| view.shared_edits = Some(shared.clone()));
    assert!(cx.debug_bounds("tabs-position-reason").is_some());
    assert!(
        cx.debug_bounds("tabs-position-sub").is_some(),
        "it depends on Position"
    );
    click(cx, "tabs-position-control-0");
    cx.run_until_parked();
    assert!(
        shared.lock().unwrap().is_empty(),
        "Herdr's file is not touched"
    );

    view.update(cx, |view, cx| {
        view.config.tabs_style.placement = TabPlacement::Herdr;
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("tabs-position-reason").is_none());
    click(cx, "tabs-position-control-0");
    cx.run_until_parked();
    assert!(matches!(
        shared.lock().unwrap()[..],
        [herdr_settings::Edit::TabBarPosition(
            herdr_settings::TabBarPosition::Top
        )]
    ));
}

#[test]
fn search_finds_both_positions() {
    let ids = catalog::search("position")
        .into_iter()
        .map(|index| catalog::CATALOG[index].id)
        .collect::<Vec<_>>();
    assert!(ids.contains(&"tabs-placement"), "{ids:?}");
    assert!(ids.contains(&"tabs-position"), "{ids:?}");
    let herdr = catalog::CATALOG
        .iter()
        .find(|entry| entry.id == "tabs-position")
        .unwrap();
    assert_eq!(herdr.title, "Herdr's position");
}
