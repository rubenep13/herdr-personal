//! Failures reading a repository for the graph.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[error("Git is not installed on this host")]
    GitMissing,
    #[error(transparent)]
    Host(#[from] crate::teleport::Error),
    #[error("This folder is not inside a Git repository")]
    NotARepository,
    #[error("Git {operation} gave output the graph could not read")]
    Output { operation: &'static str },
    #[error("Git {operation} failed")]
    Script {
        operation: &'static str,
        #[source]
        source: herdr_client::Error,
    },
    // keep-sorted end
}

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;
