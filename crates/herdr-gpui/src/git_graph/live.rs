//! Keeping the graph live: a fingerprint of the refs, HEAD and the working
//! tree, taken while the tab is shown, tells when to read the repository
//! again. A remote host is checked through one long-lived shell over SSH,
//! as the host's load is, rather than a new connection every time.

use super::error::{Error, Result};
use crate::{teleport::Host, usage::Shell};
use herdr_client::shell_quote;
use std::{sync::atomic::AtomicBool, time::Duration};

/// How often a shown tab checks: this machine, and an SSH host.
pub(crate) const LOCAL_EVERY: Duration = Duration::from_secs(2);
pub(crate) const REMOTE_EVERY: Duration = Duration::from_secs(5);
/// How long one check may take before the shell is given up on.
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);

/// Whether the graph is following its repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Liveness {
    Live,
    /// The last check failed; the next one may bring it back.
    Reconnecting,
    Offline,
}

/// A step that prints a checksum of `root`'s refs, HEAD and working tree,
/// read with the graph's read-only Git. It runs in a subshell, so it can
/// run in a shell that lives on after it.
pub(crate) fn fingerprint_script(root: &str) -> String {
    format!(
        r#"(
for name in $(env | sed -n 's/^\(GIT_[A-Za-z0-9_]*\)=.*/\1/p'); do unset "$name"; done
herdr_git() {{ GIT_OPTIONAL_LOCKS=0 GIT_NO_LAZY_FETCH=1 git -c core.fsmonitor=false -c core.hooksPath=/dev/null -c protocol.allow=never -c gc.auto=0 "$@"; }}
cd -- {root} || exit 3
{{
herdr_git for-each-ref --format='%(objectname) %(refname)' refs/heads refs/remotes refs/tags refs/stash
herdr_git rev-parse -q --verify HEAD
herdr_git status --porcelain=v2 --untracked-files=all
}} 2>/dev/null | cksum
)
"#,
        root = shell_quote(root),
    )
}

/// Where checks run: through the host's script runner, or a shell kept
/// open on an SSH host.
pub(crate) enum Prober {
    Local(Host),
    Remote(Shell),
}

impl Prober {
    /// A prober for `host`: one shell over SSH for an SSH host.
    pub(crate) fn new(host: &Host) -> Result<Self> {
        Ok(match host.ssh_target() {
            Some(target) => {
                Self::Remote(Shell::connect(target).map_err(|error| Error::Live(Box::new(error)))?)
            }
            None => Self::Local(host.clone()),
        })
    }

    /// The repository at `root`'s fingerprint.
    pub(crate) fn fingerprint(&mut self, root: &str, cancelled: &AtomicBool) -> Result<String> {
        let script = fingerprint_script(root);
        let printed = match self {
            Self::Local(host) => {
                let output = host
                    .capture_limited(&script, super::git::LIMITS, cancelled)
                    .map_err(|source| Error::Script {
                        operation: "status",
                        source,
                    })?;
                String::from_utf8_lossy(&output).into_owned()
            }
            Self::Remote(shell) => {
                let output = shell
                    .run(&script, CHECK_TIMEOUT)
                    .map_err(|error| Error::Live(Box::new(error)))?;
                if !output.success {
                    return Err(Error::LiveCheck);
                }
                output.stdout
            }
        };
        Ok(printed.trim().to_owned())
    }

    /// The remote shell's process, to tell one shell from another.
    #[cfg(test)]
    pub(crate) fn shell_pid(&mut self) -> Option<String> {
        match self {
            Self::Remote(shell) => shell
                .run("echo $$", CHECK_TIMEOUT)
                .ok()
                .map(|output| output.stdout.trim().to_owned()),
            Self::Local(_) => None,
        }
    }
}
