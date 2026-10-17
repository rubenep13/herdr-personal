//! A read-only view of a repository's history, laid out in lanes as hgg does.

pub(crate) mod chips;
mod error;
pub(crate) mod layout;
pub(crate) mod model;
pub(crate) mod text;

pub(crate) use error::Error;

#[cfg(test)]
mod tests;
