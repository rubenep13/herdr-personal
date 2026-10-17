//! A Git graph tab's background thread. It opens the repository, reads its
//! pages and lays them out, so the UI thread only swaps in a finished
//! `History`. One request runs at a time; dropping the worker cancels the
//! script in flight and ends the thread without the UI waiting for it.

use super::{
    error::{Error, Result},
    git::{PAGE, Repo, Snapshot, WorktreeCounts},
    history::History,
    model::Commit,
};
use crate::teleport::Host;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

/// Requests or replies waiting at once; the view keeps one out at a time.
const QUEUE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Request {
    /// Read the refs and the first page again.
    Load,
    /// Read the next page.
    More,
}

#[derive(Debug, Clone)]
pub(crate) enum Reply {
    History(Arc<History>),
    Failed(Arc<Error>),
}

pub(crate) struct Worker {
    requests: mpsc::SyncSender<(u64, Request)>,
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
        let (requests, inbox) = mpsc::sync_channel(QUEUE);
        let (outbox, replies) = mpsc::sync_channel(QUEUE);
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        #[cfg(test)]
        let finished = Arc::new(AtomicBool::new(false));
        #[cfg(test)]
        let done = finished.clone();
        let failure = std::thread::Builder::new()
            .name("git-graph".into())
            .spawn(move || {
                run(&host, &folder, &inbox, &outbox, &flag);
                #[cfg(test)]
                done.store(true, Ordering::Release);
            })
            .err()
            .map(|error| Arc::new(Error::Thread(error)));
        Self {
            requests,
            replies,
            generation: 0,
            cancelled,
            failure,
            #[cfg(test)]
            finished,
        }
    }

    #[cfg(test)]
    pub(crate) fn finished(&self) -> Arc<AtomicBool> {
        self.finished.clone()
    }

    /// Queues `request` and returns its generation; replies to earlier
    /// generations are stale from now on.
    pub(crate) fn send(&mut self, request: Request) -> u64 {
        self.generation += 1;
        if self.failure.is_none() && self.requests.try_send((self.generation, request)).is_err() {
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
}

impl Drop for Worker {
    fn drop(&mut self) {
        // The thread ends once its requests close; this also stops the
        // script it is running now.
        self.cancelled.store(true, Ordering::Release);
    }
}

/// What the thread has read so far.
struct Loaded {
    repo: Repo,
    snapshot: Snapshot,
    commits: Vec<Commit>,
    worktree: Option<WorktreeCounts>,
    has_more: bool,
    /// Where the next page starts.
    skip: usize,
}

fn run(
    host: &Host,
    folder: &str,
    inbox: &mpsc::Receiver<(u64, Request)>,
    outbox: &mpsc::SyncSender<(u64, Reply)>,
    cancelled: &AtomicBool,
) {
    let mut loaded = None;
    while let Ok((generation, request)) = inbox.recv() {
        if cancelled.load(Ordering::Acquire) {
            return;
        }
        let reply = match handle(host, folder, &mut loaded, request, cancelled) {
            Ok(history) => Reply::History(Arc::new(history)),
            Err(error) => Reply::Failed(Arc::new(error)),
        };
        if cancelled.load(Ordering::Acquire) || outbox.send((generation, reply)).is_err() {
            return;
        }
    }
}

fn handle(
    host: &Host,
    folder: &str,
    loaded: &mut Option<Loaded>,
    request: Request,
    cancelled: &AtomicBool,
) -> Result<History> {
    match (request, loaded.as_mut()) {
        (Request::More, Some(state)) => {
            if state.has_more {
                let page = state.repo.page(&state.snapshot, state.skip, cancelled)?;
                state.commits.extend(page.commits);
                state.has_more = page.has_more;
                state.skip += PAGE;
            }
        }
        _ => {
            let repo = match loaded.take() {
                Some(state) => state.repo,
                None => Repo::open(host.clone(), folder, cancelled)?,
            };
            let snapshot = repo.snapshot(cancelled)?;
            let page = repo.page(&snapshot, 0, cancelled)?;
            // Uncommitted changes are an extra row; without them the graph
            // still shows the history, so a failed status is not fatal.
            let worktree = repo.worktree(cancelled).ok();
            *loaded = Some(Loaded {
                repo,
                snapshot,
                commits: page.commits,
                worktree,
                has_more: page.has_more,
                skip: PAGE,
            });
        }
    }
    let state = loaded.as_ref().ok_or(Error::WorkerStopped)?;
    let mut history = History::new(
        &state.commits,
        &state.snapshot.refs,
        state.snapshot.head.clone(),
        state.worktree,
        state.has_more,
    );
    state.repo.root().clone_into(&mut history.root);
    Ok(history)
}
