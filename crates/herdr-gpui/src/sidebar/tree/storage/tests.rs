#![allow(clippy::unwrap_used)]

use super::*;
use crate::sidebar::tree::{
    SnapshotKey,
    preferences::SpaceOrder,
    tests::{context, key},
};

fn ranked() -> TreePreferences {
    TreePreferences {
        space_orders: vec![SpaceOrder {
            context: context(&key("local", "local"), "boot"),
            workspaces: vec!["w2".into(), "w1".into()],
        }],
        ..Default::default()
    }
}

fn kept(state: &TreeState) -> Option<PathBuf> {
    match state.notice() {
        Some(Notice::Unreadable { kept, .. }) => kept.clone(),
        other => panic!("expected an unreadable notice, got {other:?}"),
    }
}

/// Moves the space order to `workspaces` in the installed state.
fn rank(cx: &mut gpui::TestAppContext, workspaces: &[&str]) {
    cx.update(|cx| {
        TreeState::update(cx, |preferences| {
            preferences.space_orders = vec![SpaceOrder {
                context: context(&key("local", "local"), "boot"),
                workspaces: workspaces.iter().map(|id| (*id).to_owned()).collect(),
            }];
            (true, ())
        })
    });
}

#[test]
fn a_missing_file_starts_empty_and_a_written_one_reads_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(FILE);
    let state = TreeState::at(Some(path.clone()));
    assert!(state.notice().is_none());
    assert_eq!(state.preferences(), &TreePreferences::default());
    write(&path, &ranked()).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(r#""version":1"#), "{text}");
    // No snapshot or terminal content is ever stored, only IDs.
    assert!(!text.contains("API") && !text.contains("label"), "{text}");
    assert_eq!(TreeState::at(Some(path)).preferences(), &ranked());
}

#[test]
fn unusable_files_are_kept_aside_rather_than_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    for (name, bytes) in [
        (
            "truncated",
            br#"{"version":1,"space_orders":[{"con"#.to_vec(),
        ),
        ("future", br#"{"version":2,"space_orders":[]}"#.to_vec()),
        ("unversioned", br#"{"space_orders":[]}"#.to_vec()),
        ("unknown", br#"{"version":1,"space_order":[]}"#.to_vec()),
        ("list", b"[1, 2]".to_vec()),
        ("huge", vec![b' '; MAX_FILE_BYTES as usize + 1]),
    ] {
        let path = dir.path().join(name).join(FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let state = TreeState::at(Some(path.clone()));
        assert_eq!(state.preferences(), &TreePreferences::default(), "{name}");
        let kept = kept(&state).unwrap_or_else(|| panic!("{name} was not kept"));
        assert_eq!(std::fs::read(&kept).unwrap(), bytes, "{name}");
        assert!(!path.exists(), "{name}");
        assert!(state.writable, "{name}");
        assert!(state.notice().unwrap().text().contains("could not be read"));
    }
    let path = dir.path().join("future").join(FILE);
    std::fs::write(&path, br#"{"version":2}"#).unwrap();
    let Some(Notice::Unreadable { error, .. }) = TreeState::at(Some(path)).notice else {
        panic!("expected an unreadable notice");
    };
    assert!(matches!(
        *error,
        crate::Error::Tree(Error::UnknownVersion(2))
    ));
}

#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_read_is_never_written_over() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(FILE);
    write(&path, &ranked()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    // Root reads anything; there the file is simply restored.
    if std::fs::read(&path).is_ok() {
        return;
    }
    let mut state = TreeState::at(Some(path.clone()));
    assert_eq!(kept(&state), None);
    assert!(!state.writable);
    assert!(state.write_job().is_none());
    assert!(state.notice().unwrap().text().contains("not saved"));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(read(&path).unwrap(), ranked());
}

#[test]
fn writes_over_the_size_limit_keep_the_previous_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(FILE);
    write(&path, &ranked()).unwrap();
    let mut huge = ranked();
    huge.space_orders[0].workspaces = (0..1200).map(|id| format!("{id:01000}")).collect();
    assert!(huge.validate().is_ok());
    assert!(matches!(
        write(&path, &huge),
        Err(crate::Error::StateFileSize { .. })
    ));
    assert_eq!(read(&path).unwrap(), ranked());
}

#[gpui::test]
fn changes_save_in_the_background_with_the_latest_value_winning(cx: &mut gpui::TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join(FILE);
    cx.update(|cx| TreeState::at(Some(path.clone())).install(cx));
    rank(cx, &["w1"]);
    rank(cx, &["w2"]);
    rank(cx, &["w3", "w1"]);
    cx.run_until_parked();
    let saved = read(&path).unwrap();
    assert_eq!(saved.space_orders[0].workspaces, ["w3", "w1"]);
    cx.update(|cx| {
        let state = TreeState::of(cx).unwrap();
        assert!(state.notice().is_none());
        assert!(!state.saving && !state.dirty);
        assert_eq!(state.sequence, 2, "the middle value was never written");
    });
    // An unchanged value is not written again.
    cx.update(|cx| TreeState::update(cx, |_| (false, ())));
    cx.run_until_parked();
    cx.update(|cx| assert_eq!(TreeState::of(cx).unwrap().sequence, 2));
}

#[gpui::test]
fn a_failed_write_is_reported_and_cleared_by_the_next_success(cx: &mut gpui::TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(FILE);
    cx.update(|cx| TreeState::at(Some(path.clone())).install(cx));
    // A directory where the file should be: every write fails.
    std::fs::create_dir(&path).unwrap();
    rank(cx, &["w1"]);
    cx.run_until_parked();
    cx.update(|cx| {
        let notice = TreeState::of(cx).unwrap().notice().unwrap();
        assert!(matches!(notice, Notice::Unsaved(_)), "{notice:?}");
        assert!(notice.text().contains("could not be saved"));
        // The order still applies in memory.
        let preferences = TreeState::of(cx).unwrap().preferences();
        assert_eq!(preferences.space_orders[0].workspaces, ["w1"]);
    });
    std::fs::remove_dir(&path).unwrap();
    rank(cx, &["w2"]);
    cx.run_until_parked();
    cx.update(|cx| assert!(TreeState::of(cx).unwrap().notice().is_none()));
    assert_eq!(read(&path).unwrap().space_orders[0].workspaces, ["w2"]);
}

#[gpui::test]
fn fixtures_without_a_state_directory_keep_changes_in_memory(cx: &mut gpui::TestAppContext) {
    rank(cx, &["w1"]);
    cx.run_until_parked();
    cx.update(|cx| {
        let state = TreeState::of(cx).unwrap();
        assert!(state.notice().is_none());
        assert_eq!(state.preferences().space_orders.len(), 1);
        let context: &SnapshotKey = &state.preferences().space_orders[0].context;
        assert_eq!(context.boot_id, "boot");
    });
}
