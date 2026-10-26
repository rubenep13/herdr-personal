use super::*;
use crate::code_view::render::split_path;
use gpui::{px, size};

fn open(path: std::path::PathBuf, view: &Entity<HerdrWindow>, cx: &mut VisualTestContext) {
    let target = EditorTarget {
        path,
        line: Some(1),
    };
    cx.update(|window, cx| view.update(cx, |view, cx| view.open_code_view(&target, window, cx)));
    cx.run_until_parked();
    // The outline follows the tab's width, which the first frame measures.
    draw(cx);
    draw(cx);
}

fn rust_file(dir: &std::path::Path) -> std::path::PathBuf {
    let folder = dir.join("src");
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("lib.rs");
    std::fs::write(
        &path,
        "pub struct Pane;\n\nimpl Pane {\n    fn width(&self) {}\n}\n",
    )
    .unwrap();
    path
}

#[test]
fn a_path_splits_into_its_name_and_folder() {
    for (path, parts) in [
        ("lib.rs", ("lib.rs", None)),
        ("/a/b.rs", ("b.rs", Some("/a"))),
        ("/b.rs", ("b.rs", Some("/"))),
        ("~/x/y/z.rs", ("z.rs", Some("~/x/y"))),
    ] {
        assert_eq!(split_path(path), parts, "{path}");
    }
}

#[gpui::test]
fn the_code_bar_names_the_file_and_its_folder(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (view, cx) = window(cx);
    open(rust_file(dir.path()), &view, cx);
    let bar = cx.debug_bounds("code-bar").unwrap();
    assert_eq!(bar.size.height, px(40.));
    let toggle = cx.debug_bounds("code-outline-toggle").unwrap();
    let editor = cx.debug_bounds("code-open-editor").unwrap();
    assert_eq!(toggle.size.height, px(26.));
    assert!(toggle.right() <= editor.left());
}

#[gpui::test]
fn outline_rows_keep_off_the_panel_edges(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (view, cx) = window(cx);
    open(rust_file(dir.path()), &view, cx);
    let panel = cx.debug_bounds("code-outline").unwrap();
    let row = cx.debug_bounds("code-outline-0").unwrap();
    let inset = row.left() - panel.left();
    assert!(inset >= px(8.) && inset <= px(9.), "{inset:?}");
    assert_eq!(row.size.height, px(32.));
    assert!(cx.debug_bounds("code-outline-header").unwrap().size.height == px(40.));
}

#[gpui::test]
fn a_read_error_is_a_dialog_error(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (view, cx) = window(cx);
    open(dir.path().join("gone.rs"), &view, cx);
    let bar = cx.debug_bounds("code-bar").unwrap();
    let error = cx.debug_bounds("code-error").unwrap();
    assert_eq!(bar.size.height, px(40.));
    assert!(error.top() >= bar.bottom());
    assert!(
        cx.debug_bounds("code-error-icon").is_some(),
        "the error's box"
    );
}

#[gpui::test]
fn buttons_stay_in_a_narrow_bar(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (view, cx) = window(cx);
    cx.simulate_resize(size(px(680.), px(560.)));
    view.update(cx, |view, _| view.config.ui.size = 18.);
    open(rust_file(dir.path()), &view, cx);
    let bar = cx.debug_bounds("code-bar").unwrap();
    let editor = cx.debug_bounds("code-open-editor").unwrap();
    assert_eq!(bar.size.height, px(60.));
    assert!(editor.right() <= bar.right(), "{editor:?} in {bar:?}");
}
