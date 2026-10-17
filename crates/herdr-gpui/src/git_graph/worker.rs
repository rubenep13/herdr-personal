//! A Git graph tab's background thread. It opens the repository, reads its
//! pages, details and diffs and lays the history out, so the UI thread only
//! swaps in finished results. One script runs at a time.
//!
//! Requests wait in a mailbox that keeps the newest of each kind (history,
//! detail, diff), the way the app's other workers keep only the latest
//! value: a selection moved quickly or a host slow to answer never piles up
//! work nobody wants any more. Dropping the worker cancels the script in
//! flight and ends the thread without the UI waiting for it.

use super::{
    detail::CommitDetail,
    error::{Error, Result},
    git::{PAGE, Repo, Snapshot, WorktreeCounts},
    history::History,
    model::Commit,
    worktree::{Section, WorktreeStatus},
};
use crate::{
    review::{Lines, highlight::Span},
    teleport::Host,
};
use std::sync::{
    Arc, Mutex, PoisonError,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

/// Replies waiting at once; the view drains them every tick.
const REPLIES: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Request {
    /// Read the refs and the first page again.
    Load,
    /// Read the next page.
    More,
    /// A commit's message and files.
    Detail(String),
    /// One file of a commit, renamed from `old_path`.
    Diff {
        oid: String,
        path: String,
        old_path: Option<String>,
    },
    /// The uncommitted files and the upstream.
    Worktree,
    /// One uncommitted file as it is in `section`.
    WorktreeDiff {
        section: Section,
        path: String,
        old_path: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum Reply {
    History(Arc<History>),
    Detail(Arc<CommitDetail>),
    Diff {
        oid: String,
        path: String,
        lines: Arc<Lines>,
        /// Each line's syntax colours.
        spans: Arc<Vec<Vec<Span>>>,
        truncated: bool,
    },
    Worktree(Arc<WorktreeStatus>),
    WorktreeDiff {
        section: Section,
        path: String,
        lines: Arc<Lines>,
        spans: Arc<Vec<Vec<Span>>>,
        truncated: bool,
    },
    /// The request of this generation failed, whatever its kind.
    Failed(Arc<Error>),
}

/// The newest request of each kind, waiting for the thread. A commit's
/// detail and the uncommitted changes' share a kind, as do their diffs:
/// the panel shows one at a time.
#[derive(Debug, Default)]
pub(crate) struct Pending {
    history: Option<(u64, Request)>,
    detail: Option<(u64, Request)>,
    diff: Option<(u64, Request)>,
}

impl Pending {
    pub(crate) fn put(&mut self, generation: u64, request: Request) {
        match request {
            Request::Load | Request::More => {
                // A reload waiting stands for a page asked for after it.
                let reload = matches!(self.history, Some((_, Request::Load)));
                let request = if reload { Request::Load } else { request };
                self.history = Some((generation, request));
            }
            Request::Detail(_) | Request::Worktree => self.detail = Some((generation, request)),
            Request::Diff { .. } | Request::WorktreeDiff { .. } => {
                self.diff = Some((generation, request));
            }
        }
    }

    /// Everything waiting, history first.
    pub(crate) fn take(&mut self) -> Vec<(u64, Request)> {
        [self.history.take(), self.detail.take(), self.diff.take()]
            .into_iter()
            .flatten()
            .collect()
    }
}

pub(crate) struct Worker {
    pending: Arc<Mutex<Pending>>,
    wake: mpsc::SyncSender<()>,
    replies: mpsc::Receiver<(u64, Reply)>,
    /// The generation of the last request sent.
    generation: u64,
    cancelled: Arc<AtomicBool>,
    /// Why the thread is not there, reported once.
    failure: Option<Arc<Error>>,
    /// Set when the thread ends, for tests that close a tab.
    #[cfg(test)]
    finished: Arc<AtomicBool>,
}

impl Worker {
    pub(crate) fn start(host: Host, folder: String) -> Self {
        let pending = Arc::new(Mutex::new(Pending::default()));
        let mailbox = pending.clone();
        let (wake, woken) = mpsc::sync_channel(1);
        let (outbox, replies) = mpsc::sync_channel(REPLIES);
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        #[cfg(test)]
        let finished = Arc::new(AtomicBool::new(false));
        #[cfg(test)]
        let done = finished.clone();
        let failure = std::thread::Builder::new()
            .name("git-graph".into())
            .spawn(move || {
                run(&host, &folder, &mailbox, &woken, &outbox, &flag);
                #[cfg(test)]
                done.store(true, Ordering::Release);
            })
            .err()
            .map(|error| Arc::new(Error::Thread(error)));
        Self {
            pending,
            wake,
            replies,
            generation: 0,
            cancelled,
            failure,
            #[cfg(test)]
            finished,
        }
    }

    /// Hands `request` to the thread and returns its generation; replies to
    /// earlier generations of its kind are stale from now on.
    pub(crate) fn send(&mut self, request: Request) -> u64 {
        self.generation += 1;
        if self.failure.is_some() {
            return self.generation;
        }
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .put(self.generation, request);
        // A full wake-up queue already has the thread coming.
        if let Err(mpsc::TrySendError::Disconnected(())) = self.wake.try_send(()) {
            self.failure = Some(Arc::new(Error::WorkerStopped));
        }
        self.generation
    }

    pub(crate) fn try_recv(&mut self) -> Option<(u64, Reply)> {
        if let Some(failure) = self.failure.take() {
            return Some((self.generation, Reply::Failed(failure)));
        }
        self.replies.try_recv().ok()
    }

    #[cfg(test)]
    pub(crate) fn finished(&self) -> Arc<AtomicBool> {
        self.finished.clone()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        // The thread ends once its wake-ups close; this also stops the
        // script it is running now.
        self.cancelled.store(true, Ordering::Release);
    }
}

/// The history the thread has read so far.
struct Loaded {
    snapshot: Snapshot,
    commits: Vec<Commit>,
    worktree: Option<WorktreeCounts>,
    has_more: bool,
    /// Where the next page starts.
    skip: usize,
}

/// What the thread keeps between requests.
struct State<'a> {
    host: &'a Host,
    folder: &'a str,
    repo: Option<Repo>,
    loaded: Option<Loaded>,
}

fn run(
    host: &Host,
    folder: &str,
    pending: &Mutex<Pending>,
    woken: &mpsc::Receiver<()>,
    outbox: &mpsc::SyncSender<(u64, Reply)>,
    cancelled: &AtomicBool,
) {
    let mut state = State {
        host,
        folder,
        repo: None,
        loaded: None,
    };
    while woken.recv().is_ok() {
        let batch = pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        for (generation, request) in batch {
            if cancelled.load(Ordering::Acquire) {
                return;
            }
            let reply = state
                .handle(request, cancelled)
                .unwrap_or_else(|error| Reply::Failed(Arc::new(error)));
            if cancelled.load(Ordering::Acquire) || outbox.send((generation, reply)).is_err() {
                return;
            }
        }
    }
}

impl State<'_> {
    fn repo(&mut self, cancelled: &AtomicBool) -> Result<&Repo> {
        if self.repo.is_none() {
            self.repo = Some(Repo::open(self.host.clone(), self.folder, cancelled)?);
        }
        self.repo.as_ref().ok_or(Error::WorkerStopped)
    }

    fn handle(&mut self, request: Request, cancelled: &AtomicBool) -> Result<Reply> {
        match request {
            Request::Detail(oid) => {
                let detail = self.repo(cancelled)?.commit_detail(&oid, cancelled)?;
                Ok(Reply::Detail(Arc::new(detail)))
            }
            Request::Diff {
                oid,
                path,
                old_path,
            } => {
                let diff =
                    self.repo(cancelled)?
                        .file_diff(&oid, &path, old_path.as_deref(), cancelled)?;
                Ok(Reply::Diff {
                    oid,
                    path,
                    lines: Arc::new(diff.lines),
                    spans: Arc::new(diff.spans),
                    truncated: diff.truncated,
                })
            }
            Request::Worktree => {
                let status = self.repo(cancelled)?.worktree_status(cancelled)?;
                Ok(Reply::Worktree(Arc::new(status)))
            }
            Request::WorktreeDiff {
                section,
                path,
                old_path,
            } => {
                let diff = self.repo(cancelled)?.worktree_diff(
                    section,
                    &path,
                    old_path.as_deref(),
                    cancelled,
                )?;
                Ok(Reply::WorktreeDiff {
                    section,
                    path,
                    lines: Arc::new(diff.lines),
                    spans: Arc::new(diff.spans),
                    truncated: diff.truncated,
                })
            }
            Request::More if self.loaded.is_some() => self.more(cancelled),
            Request::Load | Request::More => self.load(cancelled),
        }
    }

    fn load(&mut self, cancelled: &AtomicBool) -> Result<Reply> {
        let repo = self.repo(cancelled)?;
        let snapshot = repo.snapshot(cancelled)?;
        let page = repo.page(&snapshot, 0, cancelled)?;
        // Uncommitted changes are an extra row; without them the graph
        // still shows the history, so a failed status is not fatal.
        let worktree = repo.worktree(cancelled).ok();
        self.loaded = Some(Loaded {
            snapshot,
            commits: page.commits,
            worktree,
            has_more: page.has_more,
            skip: PAGE,
        });
        self.history()
    }

    fn more(&mut self, cancelled: &AtomicBool) -> Result<Reply> {
        let repo = self.repo(cancelled)?.clone();
        let loaded = self.loaded.as_mut().ok_or(Error::WorkerStopped)?;
        if loaded.has_more {
            let page = repo.page(&loaded.snapshot, loaded.skip, cancelled)?;
            loaded.commits.extend(page.commits);
            loaded.has_more = page.has_more;
            loaded.skip += PAGE;
        }
        self.history()
    }

    fn history(&self) -> Result<Reply> {
        let loaded = self.loaded.as_ref().ok_or(Error::WorkerStopped)?;
        let mut history = History::new(
            &loaded.commits,
            &loaded.snapshot.refs,
            loaded.snapshot.head.clone(),
            loaded.worktree,
            loaded.has_more,
        );
        if let Some(repo) = &self.repo {
            repo.root().clone_into(&mut history.root);
        }
        Ok(Reply::History(Arc::new(history)))
    }
}
