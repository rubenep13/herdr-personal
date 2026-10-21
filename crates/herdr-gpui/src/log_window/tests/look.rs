use super::*;
use crate::design::Tone;

fn console(cx: &mut TestAppContext, rows: usize) -> (Entity<LogWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(move |window, cx| {
        let mut view = LogWindow::new(window, cx);
        view.following = false;
        view.generation = Some(diagnostics::generation());
        view.retained = (0..rows)
            .map(|index| Arc::new(Record::fixture(Level::INFO, format!("fixture row {index}"))))
            .collect();
        view.rows = view.retained.clone();
        view.scroll.reset(view.rows.len());
        view
    });
    cx.simulate_resize(size(px(1100.), px(650.)));
    draw(cx);
    (view, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

#[gpui::test]
fn the_logs_bar_holds_every_control(cx: &mut TestAppContext) {
    let (_view, cx) = console(cx, 3);
    let bar = cx.debug_bounds("logs-bar").unwrap();
    assert_eq!(bar.size.height, px(40.));
    let mut left = bar.left();
    for selector in ["logs-search", "minimum-level", "follow", "copy", "export"] {
        let control = cx.debug_bounds(selector).unwrap();
        assert!(control.left() >= left, "{selector} is out of order");
        assert!(control.top() >= bar.top() && control.bottom() <= bar.bottom());
        assert!(control.right() <= bar.right(), "{selector}");
        left = control.right();
    }
    let search = cx.debug_bounds("theme-search").unwrap();
    let field = cx.debug_bounds("logs-search").unwrap();
    assert!(field.contains(&search.center()));
}

#[gpui::test]
fn the_level_menu_marks_the_chosen_level(cx: &mut TestAppContext) {
    let (view, cx) = console(cx, 3);
    let minimum = view.read_with(cx, |view, _| view.minimum);
    let dropdown = cx.debug_bounds("minimum-level").unwrap();
    cx.simulate_click(dropdown.center(), Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("level-menu").is_some());
    for (index, level) in LEVELS.iter().enumerate() {
        let row = format!("level-option-{index}").leak();
        let check = format!("level-option-{index}-check").leak();
        assert_eq!(cx.debug_bounds(row).unwrap().size.height, px(32.));
        assert_eq!(
            cx.debug_bounds(check).is_some(),
            *level == minimum,
            "{level}"
        );
    }
}

#[test]
fn level_chips_follow_their_severity() {
    for (level, tone) in [
        (Level::ERROR, Tone::Danger),
        (Level::WARN, Tone::Warn),
        (Level::INFO, Tone::Accent),
        (Level::DEBUG, Tone::Neutral),
        (Level::TRACE, Tone::Neutral),
    ] {
        assert_eq!(level_tone(level), tone, "{level}");
    }
}

#[gpui::test]
fn a_long_log_draws_only_what_shows(cx: &mut TestAppContext) {
    let drawn = |cx: &mut VisualTestContext, rows: usize| {
        (0..rows)
            .filter(|index| cx.debug_bounds(format!("log-row-{index}").leak()).is_some())
            .count()
    };
    let few = {
        let (_short, cx) = console(cx, 200);
        drawn(cx, 200)
    };
    let (_long, cx) = console(cx, 5000);
    let many = drawn(cx, 5000);
    assert!(few > 0 && few < 200, "{few}");
    assert_eq!(few, many);
}

#[gpui::test]
fn a_narrow_console_keeps_every_button(cx: &mut TestAppContext) {
    let mut config = Config::default();
    config.ui.size = 16.;
    cx.update(|cx| set_appearance(&config, &Theme::default(), cx));
    let (_view, cx) = console(cx, 3);
    cx.simulate_resize(size(px(620.), px(360.)));
    draw(cx);
    let bar = cx.debug_bounds("logs-bar").unwrap();
    assert!((bar.size.height - px(40. * 16. / 12.)).abs() < px(1.));
    let field = cx.debug_bounds("logs-search").unwrap();
    assert!(field.size.width > px(100.), "{field:?}");
    for selector in ["minimum-level", "follow", "copy", "export"] {
        let control = cx.debug_bounds(selector).unwrap();
        assert!(control.right() <= bar.right(), "{selector}: {control:?}");
    }
}
