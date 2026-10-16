//! Where the tree's order lives between runs: one versioned file in this
//! client's own state directory, shared by every window of the app.
//!
//! It is read once before GPUI starts, like the worktree notes. Each change
//! is written on the background executor, at most one write at a time, the
//! latest value replacing any still waiting. A file this client cannot use is
//! moved aside and named in a notice instead of being overwritten, and a
//! failed write keeps the previous file and says so; either way the sidebar
//! keeps working with what it has in memory.

use super::{Error, TreePreferences};
use crate::state_file;
use gpui::{App, Global};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub(crate) const FILE: &str = "sidebar-tree-v1.json";
/// Larger files are refused on load and never written.
pub(crate) const MAX_FILE_BYTES: u64 = 1024 * 1024;
const VERSION: u64 = 1;

/// Why the stored order is not what the user last arranged.
#[derive(Clone, Debug)]
pub(crate) enum Notice {
    /// The file could not be used. `kept` is where it was moved aside; while
    /// it could not be, nothing is written over it this run.
    Unreadable {
        error: Arc<crate::Error>,
        kept: Option<PathBuf>,
    },
    /// The latest change was not saved; the file holds an earlier order.
    Unsaved(Arc<crate::Error>),
}

impl Notice {
    pub(crate) fn text(&self) -> String {
        match self {
            Self::Unreadable {
                error,
                kept: Some(kept),
            } => format!(
                "The saved sidebar order could not be read ({error}); it was kept as {}.",
                kept.display()
            ),
            Self::Unreadable { error, kept: None } => format!(
                "The saved sidebar order could not be read ({error}); changes are not saved this session."
            ),
            Self::Unsaved(error) => format!("The sidebar order could not be saved ({error})."),
        }
    }
}

#[derive(Default)]
pub(crate) struct TreeState {
    preferences: TreePreferences,
    path: Option<PathBuf>,
    notice: Option<Notice>,
    /// Whether a write may replace the file at `path`.
    writable: bool,
    saving: bool,
    /// A change arrived while a write was in flight.
    dirty: bool,
    quitting: bool,
    /// The sequence of the newest value written, so a write that loses a race
    /// with a newer one never lands after it.
    written: Arc<Mutex<u64>>,
    sequence: u64,
}

impl Global for TreeState {}

impl TreeState {
    /// Called before starting GPUI. A missing file starts with the daemon's
    /// order.
    pub(crate) fn load() -> Self {
        Self::at(crate::preferences::state_dir().map(|dir| dir.join(FILE)))
    }

    fn at(path: Option<PathBuf>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        let (preferences, notice, writable) = match read(&path) {
            Ok(preferences) => (preferences, None, true),
            Err(error) => {
                let kept = damaged(&error).then(|| set_aside(&path)).flatten();
                tracing::warn!(%error, kept = kept.is_some(), "Cannot restore the sidebar tree");
                let writable = kept.is_some();
                let notice = Notice::Unreadable {
                    error: Arc::new(error),
                    kept,
                };
                (TreePreferences::default(), Some(notice), writable)
            }
        };
        Self {
            preferences,
            path: Some(path),
            notice,
            writable,
            ..Self::default()
        }
    }

    pub(crate) fn install(self, cx: &mut App) {
        cx.set_global(self);
        cx.on_app_quit(|cx| {
            let state = cx.global_mut::<Self>();
            state.quitting = true;
            let pending = (state.dirty || state.saving)
                .then(|| state.write_job())
                .flatten();
            cx.background_executor().spawn(async move {
                if let Some(job) = pending {
                    let _ = job();
                }
            })
        })
        .detach();
    }

    pub(crate) fn of(cx: &App) -> Option<&Self> {
        cx.try_global::<Self>()
    }

    pub(crate) fn preferences(&self) -> &TreePreferences {
        &self.preferences
    }

    /// Whether the user folded host `endpoint_id` when last it was listed.
    pub(crate) fn host_collapsed(cx: &App, endpoint_id: &str) -> bool {
        Self::of(cx).is_some_and(|state| state.preferences.collapsed_hosts.contains(endpoint_id))
    }

    pub(crate) fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    /// Changes the stored order through `f`, which returns whether it changed
    /// anything, and saves it if so. Fixtures that never installed the state
    /// get an unsaved one.
    pub(crate) fn update<R>(cx: &mut App, f: impl FnOnce(&mut TreePreferences) -> (bool, R)) -> R {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::default());
        }
        let state = cx.global_mut::<Self>();
        let (changed, result) = f(&mut state.preferences);
        if changed {
            Self::save(cx);
        }
        result
    }

    /// Forgets the ranks and folds each endpoint's new snapshot makes stale,
    /// such as an earlier boot's. Leaves the state untouched, and its
    /// observers quiet, when there is nothing to forget.
    pub(crate) fn reconcile(
        cx: &mut App,
        snapshots: &[(
            super::EndpointKey,
            Arc<herdr_client::protocol::ClientShellSnapshot>,
        )],
    ) {
        let Some(state) = Self::of(cx) else {
            return;
        };
        let mut next = None::<TreePreferences>;
        for (key, snapshot) in snapshots {
            if !state.preferences.mentions(key) {
                continue;
            }
            let preferences = next.get_or_insert_with(|| state.preferences.clone());
            preferences.reconcile(key, snapshot);
        }
        let Some(next) = next.filter(|next| *next != state.preferences) else {
            return;
        };
        Self::update(cx, |preferences| {
            *preferences = next;
            (true, ())
        });
    }

    /// Clears a notice the user has read.
    pub(crate) fn dismiss(cx: &mut App) {
        if let Some(state) = cx.try_global::<Self>()
            && state.notice.is_some()
        {
            cx.global_mut::<Self>().notice = None;
        }
    }

    fn save(cx: &mut App) {
        let state = cx.global_mut::<Self>();
        if state.quitting {
            return;
        }
        if state.saving {
            state.dirty = true;
            return;
        }
        let Some(job) = state.write_job() else {
            return;
        };
        state.saving = true;
        let task = cx.background_executor().spawn(async move { job() });
        cx.spawn(async move |cx| {
            let result = task.await;
            cx.update(|cx| Self::saved(cx, result));
        })
        .detach();
    }

    fn saved(cx: &mut App, result: crate::Result<()>) {
        let state = cx.global_mut::<Self>();
        state.saving = false;
        match result {
            Ok(()) => {
                if matches!(state.notice, Some(Notice::Unsaved(_))) {
                    state.notice = None;
                }
            }
            Err(error) => {
                tracing::warn!(%error, "Cannot save the sidebar tree");
                state.notice = Some(Notice::Unsaved(Arc::new(error)));
            }
        }
        if std::mem::take(&mut state.dirty) {
            Self::save(cx);
        }
    }

    /// The write of the current value, to run off the UI thread. `None` when
    /// nothing may be written.
    fn write_job(&mut self) -> Option<impl FnOnce() -> crate::Result<()> + Send + 'static> {
        let path = self.path.clone().filter(|_| self.writable)?;
        self.sequence += 1;
        let sequence = self.sequence;
        let value = self.preferences.clone();
        let written = self.written.clone();
        Some(move || {
            // Held through the write, so writes land in sequence order.
            let mut newest = written
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if *newest > sequence {
                return Ok(());
            }
            write(&path, &value)?;
            *newest = sequence;
            Ok(())
        })
    }
}

/// The stored order, or defaults when there is no file yet.
pub(crate) fn read(path: &Path) -> crate::Result<TreePreferences> {
    let Some(bytes) = state_file::read(path, MAX_FILE_BYTES)? else {
        return Ok(TreePreferences::default());
    };
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let object = value
        .as_object_mut()
        .ok_or(crate::Error::PreferencesNotObject)?;
    match object.remove("version").map(|version| version.as_u64()) {
        Some(Some(VERSION)) => {}
        Some(Some(version)) => return Err(Error::UnknownVersion(version).into()),
        _ => return Err(Error::Unversioned.into()),
    }
    let preferences: TreePreferences = serde_json::from_value(value)?;
    preferences.validate()?;
    Ok(preferences)
}

/// Writes `preferences` atomically, refusing a file over [`MAX_FILE_BYTES`]
/// so the previous one stays.
pub(crate) fn write(path: &Path, preferences: &TreePreferences) -> crate::Result<()> {
    preferences.validate()?;
    let mut value = serde_json::to_value(preferences)?;
    if let Some(object) = value.as_object_mut() {
        object.insert("version".into(), VERSION.into());
    }
    let size = serde_json::to_vec(&value)?.len() as u64;
    if size > MAX_FILE_BYTES {
        return Err(crate::Error::StateFileSize {
            path: path.to_owned(),
            limit: MAX_FILE_BYTES,
        });
    }
    state_file::write(path, &value)
}

/// Whether the file exists but holds nothing this client can use, as
/// opposed to one it could not read at all.
fn damaged(error: &crate::Error) -> bool {
    !matches!(error, crate::Error::Io(_))
}

/// Moves an unusable file aside, so the next write cannot destroy it.
fn set_aside(path: &Path) -> Option<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let mut name = path.file_name()?.to_owned();
    name.push(format!(".unreadable-{stamp}"));
    let kept = path.with_file_name(name);
    match std::fs::rename(path, &kept) {
        Ok(()) => Some(kept),
        Err(error) => {
            tracing::warn!(%error, "Cannot move the unreadable sidebar tree aside");
            None
        }
    }
}

#[cfg(test)]
mod tests;
