//! Failures reading a repository for the graph.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[error("Git is not installed on this host")]
    GitMissing,
    #[error(transparent)]
    Host(#[from] crate::teleport::Error),
    #[error("A Git graph needs an absolute folder")]
    InvalidCheckout,
    #[error("This folder is not inside a Git repository")]
    NotARepository,
    #[error("Git {operation} gave output the graph could not read")]
    Output { operation: &'static str },
    #[error("the saved Git graph settings have no version")]
    PrefsUnversioned,
    #[error("the saved Git graph settings are version {0}, which this client does not know")]
    PrefsVersion(u64),
    #[error("Git {operation} failed")]
    Script {
        operation: &'static str,
        #[source]
        source: herdr_client::Error,
    },
    #[error("Could not start the Git graph's reader")]
    Thread(#[source] std::io::Error),
    #[error("The Git graph's reader stopped")]
    WorkerStopped,
    // keep-sorted end
}

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;
