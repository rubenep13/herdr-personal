#![allow(clippy::expect_used, clippy::unwrap_used)]

#[cfg(unix)]
use super::git::Repo;
use super::model::{Commit, CommitKind};
#[cfg(unix)]
use crate::teleport::Host;
#[cfg(unix)]
use herdr_client::ConnectTarget;
#[cfg(unix)]
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

mod chips;
mod detail;
#[cfg(unix)]
mod git;
mod keys;
mod layout;
mod prefs;
#[cfg(unix)]
mod readonly;
mod remote;
mod rows;
mod tab;
mod text;
mod worker;

/// A commit with only the fields the lane engine reads.
fn commit(oid: &str, parents: &[&str]) -> Commit {
    Commit {
        oid: oid.into(),
        parents: parents.iter().map(|parent| (*parent).into()).collect(),
        author: "Ada".into(),
        email: "ada@example.com".into(),
        authored: 0,
        subject: oid.into(),
        description: String::new(),
        kind: CommitKind::Commit,
    }
}

/// hgg's own fixture: two branches, a merge, an octopus merge and a
/// disconnected root.
fn hgg_history() -> Vec<Commit> {
    vec![
        commit("m", &["a2", "b1"]),
        commit("b1", &["a1"]),
        commit("a2", &["o"]),
        commit("o", &["a1", "c1", "d1"]),
        commit("d1", &["a1"]),
        commit("c1", &["a1"]),
        commit("a1", &["root"]),
        commit("root", &[]),
        commit("orphan2", &["orphan1"]),
        commit("orphan1", &[]),
    ]
}

/// Forty commits, child before parent, whose branches open, merge and cross.
fn tangled_history() -> Vec<Commit> {
    const COUNT: usize = 40;
    let oid = |row: usize| format!("c{row:02}");
    (0..COUNT)
        .map(|row| {
            let mut parents = Vec::new();
            if row % 7 == 0 && row + 4 < COUNT {
                parents.extend([oid(row + 2), oid(row + 4)]);
            } else if row % 3 == 0 && row + 3 < COUNT {
                parents.push(oid(row + 3));
            } else if row + 1 < COUNT {
                parents.push(oid(row + 1));
            }
            if row % 11 == 5 && row + 6 < COUNT {
                parents.push(oid(row + 6));
            }
            let parents: Vec<&str> = parents.iter().map(String::as_str).collect();
            commit(&oid(row), &parents)
        })
        .collect()
}

/// A repository in a temporary directory, driven by the system `git` with no
/// global or system configuration.
#[cfg(unix)]
struct Fixture {
    temp: tempfile::TempDir,
    repo: PathBuf,
}

#[cfg(unix)]
impl Fixture {
    /// An empty repository on `main`.
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::create_dir_all(temp.path().join("home")).unwrap();
        let fixture = Self { temp, repo };
        fixture.git(&["init", "-q", "-b", "main"]);
        fixture
    }

    /// A repository with `count` commits on `main`, the newest first.
    fn with_commits(count: usize) -> Self {
        let fixture = Self::new();
        for index in 0..count {
            fixture.write("file.txt", &format!("{index}\n"));
            fixture.commit(&format!("commit {index}"));
        }
        fixture
    }

    fn path(&self) -> &Path {
        &self.repo
    }

    fn command(&self, directory: &Path) -> Command {
        let mut command = Command::new("git");
        command
            .current_dir(directory)
            .env("HOME", self.temp.path().join("home"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
            ])
            .args(["-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false"]);
        command
    }

    /// Runs `git` in `directory` and returns its stdout.
    fn git_in(&self, directory: &Path, args: &[&str]) -> String {
        let output = self.command(directory).args(args).output().unwrap();
        assert!(output.status.success(), "git {args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap()
    }

    fn git(&self, args: &[&str]) -> String {
        self.git_in(&self.repo, args)
    }

    /// Runs `git fast-import` on `stream`.
    fn import(&self, stream: &str) {
        use std::io::Write;
        let mut child = self
            .command(&self.repo)
            .args(["fast-import", "--quiet"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stream.as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success());
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.repo.join(name), text).unwrap();
    }

    fn commit(&self, message: &str) -> String {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-qm", message]);
        self.head()
    }

    fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().to_owned()
    }

    fn host(&self) -> Host {
        Host::new(&ConnectTarget::Local).unwrap()
    }

    fn open(&self) -> Repo {
        open_at(self.host(), &self.repo)
    }
}

#[cfg(unix)]
fn open_at(host: Host, folder: &Path) -> Repo {
    Repo::open(host, folder.to_str().unwrap(), &AtomicBool::new(false)).unwrap()
}

fn look() -> super::view::GraphLook {
    let config = crate::config::Config::default();
    super::view::GraphLook {
        theme: crate::config::Theme::default(),
        ui: config.ui.clone(),
        mono: config.terminal.clone(),
    }
}

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

/// A view showing `history` in a window `height` pixels tall, with no
/// worker behind it.
fn shown(
    cx: &mut gpui::TestAppContext,
    history: super::history::History,
    height: f32,
) -> (
    gpui::Entity<super::view::GitGraphView>,
    &mut gpui::VisualTestContext,
) {
    use gpui::{px, size};
    let checkout = super::GraphCheckout::new("/repo".into()).unwrap();
    let (view, cx) = cx.add_window_view(|_, cx| {
        super::view::GitGraphView::new(
            Err(super::Error::WorkerStopped),
            checkout,
            look(),
            super::view::Connection::Online,
            cx,
        )
    });
    cx.simulate_resize(size(px(1000.), px(height)));
    cx.update(|_, cx| {
        view.update(cx, |view, cx| {
            let reply = super::worker::Reply::History(std::sync::Arc::new(history));
            assert!(view.receive(0, reply, cx));
        })
    });
    draw(cx);
    (view, cx)
}

fn bounds(cx: &mut gpui::VisualTestContext, selector: &str) -> gpui::Bounds<gpui::Pixels> {
    cx.debug_bounds(Box::leak(selector.to_owned().into_boxed_str()))
        .unwrap_or_else(|| panic!("{selector} is drawn"))
}

fn drawn(cx: &mut gpui::VisualTestContext, selector: &str) -> bool {
    cx.debug_bounds(Box::leak(selector.to_owned().into_boxed_str()))
        .is_some()
}

/// `count` commits in a line, `c0` the newest.
fn line(count: usize) -> Vec<Commit> {
    (0..count)
        .map(|row| {
            let parent = format!("c{}", row + 1);
            let parents: Vec<&str> = if row + 1 < count {
                vec![&parent]
            } else {
                vec![]
            };
            commit(&format!("c{row}"), &parents)
        })
        .collect()
}
