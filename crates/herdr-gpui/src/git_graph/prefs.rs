//! The graph's columns and widths, shared by every graph tab and kept
//! between runs in one versioned file in this client's state directory.
//!
//! It is read once before GPUI starts. Changes are saved on a worker, the
//! latest value replacing any still waiting. A file this client cannot use
//! is moved aside rather than overwritten; while it cannot be moved, nothing
//! is written this run.

use super::{error::Error, geometry::REFS_DEFAULT};
use crate::state_file;
use gpui::{App, Global};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(crate) const FILE: &str = "git-graph-v1.json";
/// Larger files are refused on load.
const MAX_FILE_BYTES: u64 = 64 * 1024;
const VERSION: u64 = 1;

/// The panel's default width.
pub(crate) const PANEL_DEFAULT: f32 = 520.;
pub(crate) const PANEL_MIN: f32 = 320.;
pub(crate) const REFS_MIN: f32 = 90.;
pub(crate) const REFS_MAX: f32 = 640.;
/// The narrowest graph a drag leaves.
pub(crate) const GRAPH_MIN: f32 = 64.;
/// A width beyond this is not a real column.
const WIDTH_MAX: f32 = 4096.;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct GraphPrefs {
    pub(crate) author: bool,
    pub(crate) date: bool,
    pub(crate) sha: bool,
    pub(crate) refs_width: f32,
    /// `None` fits the graph to its lanes.
    pub(crate) graph_width: Option<f32>,
    pub(crate) panel_width: f32,
}

impl Default for GraphPrefs {
    fn default() -> Self {
        Self {
            author: false,
            date: false,
            sha: false,
            refs_width: REFS_DEFAULT,
            graph_width: None,
            panel_width: PANEL_DEFAULT,
        }
    }
}

/// `width` within `min..=max`, or `fallback` when it is not a number.
fn within(width: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if width.is_finite() {
        width.clamp(min, max)
    } else {
        fallback
    }
}

impl GraphPrefs {
    /// The same choices with every width within its bounds.
    pub(crate) fn clamped(self) -> Self {
        Self {
            refs_width: within(self.refs_width, REFS_MIN, REFS_MAX, REFS_DEFAULT),
            graph_width: self
                .graph_width
                .filter(|width| width.is_finite())
                .map(|width| width.clamp(GRAPH_MIN, WIDTH_MAX)),
            panel_width: within(self.panel_width, PANEL_MIN, WIDTH_MAX, PANEL_DEFAULT),
            ..self
        }
    }
}

/// The stored choices, or defaults when there is no file yet.
pub(crate) fn read(path: &Path) -> crate::Result<GraphPrefs> {
    let Some(bytes) = state_file::read(path, MAX_FILE_BYTES)? else {
        return Ok(GraphPrefs::default());
    };
    let mut value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let object = value
        .as_object_mut()
        .ok_or(crate::Error::PreferencesNotObject)?;
    match object.remove("version").map(|version| version.as_u64()) {
        Some(Some(VERSION)) => {}
        Some(Some(version)) => return Err(Error::PrefsVersion(version).into()),
        _ => return Err(Error::PrefsUnversioned.into()),
    }
    let prefs: GraphPrefs = serde_json::from_value(value)?;
    Ok(prefs.clamped())
}

#[derive(Serialize)]
struct Saved {
    version: u64,
    #[serde(flatten)]
    prefs: GraphPrefs,
}

/// What the store's worker writes, for tests that read it back.
#[cfg(test)]
pub(crate) fn write(path: &Path, prefs: &GraphPrefs) -> crate::Result<()> {
    state_file::write(
        path,
        &Saved {
            version: VERSION,
            prefs: *prefs,
        },
    )
}

/// Moves an unusable file aside, so no write can destroy it.
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
            tracing::warn!(%error, "Cannot move the unreadable Git graph settings aside");
            None
        }
    }
}

#[derive(Default)]
pub(crate) struct GraphPrefsStore {
    prefs: GraphPrefs,
    writer: Option<state_file::Writer<Saved>>,
    quitting: bool,
}

impl Global for GraphPrefsStore {}

impl GraphPrefsStore {
    /// Called before starting GPUI.
    pub(crate) fn load() -> Self {
        Self::at(crate::preferences::state_dir().map(|dir| dir.join(FILE)))
    }

    pub(crate) fn at(path: Option<PathBuf>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        let (prefs, writable) = match read(&path) {
            Ok(prefs) => (prefs, true),
            Err(error) => {
                // A file that could not be read at all is left alone; one
                // that holds nothing usable is kept aside.
                let kept = !matches!(error, crate::Error::Io(_)) && set_aside(&path).is_some();
                tracing::warn!(%error, kept, "Cannot restore the Git graph settings");
                (GraphPrefs::default(), kept)
            }
        };
        let writer = writable
            .then(|| state_file::Writer::start("git-graph-settings", path))
            .and_then(|started| {
                started
                    .inspect_err(|error| {
                        tracing::warn!(%error, "Cannot start the Git graph settings worker");
                    })
                    .ok()
            });
        Self {
            prefs,
            writer,
            quitting: false,
        }
    }

    pub(crate) fn install(self, cx: &mut App) {
        cx.set_global(self);
        cx.on_app_quit(|cx| {
            let store = cx.global_mut::<Self>();
            store.quitting = true;
            let writer = store.writer.take();
            cx.background_executor().spawn(async move {
                if let Some(writer) = writer {
                    writer.finish();
                }
            })
        })
        .detach();
    }

    pub(crate) fn prefs(&self) -> GraphPrefs {
        self.prefs
    }

    /// The choices every graph tab draws with; defaults for fixtures that
    /// never installed the store.
    pub(crate) fn get(cx: &App) -> GraphPrefs {
        cx.try_global::<Self>().map(Self::prefs).unwrap_or_default()
    }

    /// Changes the choices through `f` and saves them if they changed.
    /// Fixtures that never installed the store get an unsaved one.
    pub(crate) fn update(cx: &mut App, f: impl FnOnce(&mut GraphPrefs)) {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::default());
        }
        let store = cx.global_mut::<Self>();
        let mut next = store.prefs;
        f(&mut next);
        let next = next.clamped();
        if next == store.prefs {
            return;
        }
        store.prefs = next;
        if !store.quitting
            && let Some(writer) = &store.writer
        {
            writer.save(Saved {
                version: VERSION,
                prefs: next,
            });
        }
    }
}
