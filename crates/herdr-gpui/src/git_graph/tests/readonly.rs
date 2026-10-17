//! The graph never writes to the repository it shows, ported from hgg's
//! `readonly.test.ts`: hooks, a filesystem monitor and a stale index are
//! traps a writing Git command would trip.

use super::*;
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::time::{Duration, SystemTime};

/// Every file and directory under `root` with its content and mtime.
fn fingerprint(root: &Path) -> BTreeMap<PathBuf, (Vec<u8>, SystemTime)> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            let modified = metadata.modified().unwrap();
            if metadata.is_dir() {
                found.insert(path.clone(), (Vec::new(), modified));
                pending.push(path);
            } else if metadata.is_symlink() {
                let target = std::fs::read_link(&path).unwrap();
                found.insert(
                    path,
                    (target.into_os_string().into_encoded_bytes(), modified),
                );
            } else {
                found.insert(path.clone(), (std::fs::read(&path).unwrap(), modified));
            }
        }
    }
    found
}

fn executable(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn copy_tree(from: &Path, to: &Path) {
    let status = Command::new("cp")
        .arg("-R")
        .arg(from)
        .arg(to)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn the_reader_never_writes() {
    let fixture = Fixture::with_commits(3);
    fixture.git(&["branch", "feature", "HEAD~1"]);
    fixture.write("file.txt", "stashed\n");
    fixture.git(&["stash", "push", "-q"]);
    fixture.write("file.txt", "uncommitted\n");

    // Arm the traps.
    let markers = fixture.temp.path().join("markers");
    let hooks = fixture.temp.path().join("hooks");
    std::fs::create_dir_all(&markers).unwrap();
    std::fs::create_dir_all(&hooks).unwrap();
    for hook in [
        "post-checkout",
        "reference-transaction",
        "post-index-change",
    ] {
        executable(
            &hooks.join(hook),
            &format!("#!/bin/sh\ntouch '{}/{hook}'\n", markers.display()),
        );
    }
    let monitor = fixture.temp.path().join("fsmonitor-hook");
    executable(
        &monitor,
        &format!(
            "#!/bin/sh\ntouch '{}/fsmonitor'\nexit 1\n",
            markers.display()
        ),
    );
    fixture.git(&["config", "core.hooksPath", hooks.to_str().unwrap()]);
    fixture.git(&["config", "core.fsmonitor", monitor.to_str().unwrap()]);
    let future = SystemTime::now() + Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(fixture.path().join("file.txt"))
        .unwrap()
        .set_modified(future)
        .unwrap();

    // Control: a plain status in a copy does trip them.
    let copy = fixture.temp.path().join("control");
    copy_tree(fixture.path(), &copy);
    let index = std::fs::read(copy.join(".git/index")).unwrap();
    fixture.git_in(&copy, &["status", "--porcelain"]);
    let tripped = std::fs::read(copy.join(".git/index")).unwrap() != index
        || std::fs::read_dir(&markers).unwrap().next().is_some();
    assert!(tripped, "a plain git status must trip a trap here");
    std::fs::remove_dir_all(&markers).unwrap();
    std::fs::create_dir_all(&markers).unwrap();

    let before = fingerprint(fixture.path());
    let refs = fixture.git(&["for-each-ref", "--format=%(refname) %(objectname)"]);
    let head = fixture.head();

    let cancelled = AtomicBool::new(false);
    let repo = fixture.open();
    let snapshot = repo.snapshot(&cancelled).unwrap();
    let page = repo.page(&snapshot, 0, &cancelled).unwrap();
    assert!(!page.commits.is_empty());
    let counts = repo.worktree(&cancelled).unwrap();
    assert_eq!(counts.unstaged, 1);
    let status = repo.worktree_status(&cancelled).unwrap();
    assert!(!status.files.is_empty());
    for (section, file) in &status.files {
        repo.worktree_diff(*section, &file.git_path, None, &cancelled)
            .unwrap();
    }
    let detail = repo
        .commit_detail(&page.commits[0].oid, &cancelled)
        .unwrap();
    if let Some(file) = detail.files.first() {
        repo.file_diff(&detail.commit.oid, &file.git_path, None, &cancelled)
            .unwrap();
    }

    let after = fingerprint(fixture.path());
    let changed: Vec<&PathBuf> = before
        .iter()
        .filter(|(path, value)| after.get(*path) != Some(value))
        .map(|(path, _)| path)
        .chain(after.keys().filter(|path| !before.contains_key(*path)))
        .collect();
    assert!(changed.is_empty(), "the reader changed {changed:?}");
    assert_eq!(
        fixture.git(&["for-each-ref", "--format=%(refname) %(objectname)"]),
        refs
    );
    assert_eq!(fixture.head(), head);
    let tripped: Vec<_> = std::fs::read_dir(&markers)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert!(tripped.is_empty(), "the reader ran {tripped:?}");
}

#[test]
fn remote_env_runs_the_same_scripts() {
    let fixture = Fixture::with_commits(3);
    fixture.git(&["branch", "feature", "HEAD~1"]);
    let other_home = fixture.temp.path().join("other home");
    std::fs::create_dir_all(&other_home).unwrap();
    let mut remote = fixture.host();
    remote.env = vec![("HOME".into(), other_home.to_string_lossy().into_owned())];

    let cancelled = AtomicBool::new(false);
    let local = fixture.open();
    let remote = open_at(remote, fixture.path());
    assert_eq!(local.root(), remote.root());
    let local_snapshot = local.snapshot(&cancelled).unwrap();
    let remote_snapshot = remote.snapshot(&cancelled).unwrap();
    assert_eq!(local_snapshot, remote_snapshot);
    assert_eq!(
        local.page(&local_snapshot, 0, &cancelled).unwrap(),
        remote.page(&remote_snapshot, 0, &cancelled).unwrap()
    );
}
