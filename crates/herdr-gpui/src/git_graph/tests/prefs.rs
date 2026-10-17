//! The graph's columns and widths survive a restart, and a damaged file is
//! set aside rather than overwritten.

use crate::git_graph::prefs::{FILE, GraphPrefs, GraphPrefsStore, read, write};

#[test]
fn prefs_round_trip_and_survive_a_bad_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(FILE);
    assert_eq!(read(&path).unwrap(), GraphPrefs::default(), "no file yet");

    let chosen = GraphPrefs {
        author: true,
        sha: true,
        refs_width: 250.,
        graph_width: Some(140.),
        panel_width: 600.,
        ..GraphPrefs::default()
    };
    write(&path, &chosen).unwrap();
    assert_eq!(read(&path).unwrap(), chosen);
    assert_eq!(GraphPrefsStore::at(Some(path.clone())).prefs(), chosen);

    // Out-of-range widths come back within bounds.
    std::fs::write(
        &path,
        r#"{"version":1,"author":false,"date":false,"sha":false,"refs_width":9000,"graph_width":1,"panel_width":10}"#,
    )
    .unwrap();
    let clamped = read(&path).unwrap();
    assert_eq!(
        (clamped.refs_width, clamped.graph_width, clamped.panel_width),
        (640., Some(64.), 320.)
    );

    std::fs::write(&path, b"{ not json").unwrap();
    let store = GraphPrefsStore::at(Some(path.clone()));
    assert_eq!(store.prefs(), GraphPrefs::default());
    assert!(!path.exists(), "the damaged file is moved aside");
    let kept: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        kept.iter()
            .any(|name| name.starts_with(&format!("{FILE}.unreadable-"))),
        "{kept:?}"
    );

    std::fs::write(&path, r#"{"version":7}"#).unwrap();
    assert!(read(&path).is_err(), "a newer version is not guessed at");
}
