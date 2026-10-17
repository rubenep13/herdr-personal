//! A read-only view of a repository's history, laid out in lanes as hgg does.

mod checkout;
pub(crate) mod chips;
pub(crate) mod detail;
mod error;
pub(crate) mod geometry;
pub(crate) mod git;
pub(crate) mod history;
pub(crate) mod layout;
pub(crate) mod live;
pub(crate) mod model;
mod parse;
pub(crate) mod prefs;
pub(crate) mod search;
mod tab;
pub(crate) mod text;
pub(crate) mod view;
pub(crate) mod worker;
pub(crate) mod worktree;

pub(crate) use checkout::GraphCheckout;
pub(crate) use error::Error;
pub(crate) use tab::GitGraph;
pub(crate) use view::{GitGraphView, GraphLook};

#[cfg(test)]
mod tests;
