//! A read-only view of a repository's history, laid out in lanes as hgg does.

mod checkout;
pub(crate) mod chips;
mod error;
pub(crate) mod geometry;
pub(crate) mod git;
pub(crate) mod history;
pub(crate) mod layout;
pub(crate) mod model;
mod parse;
mod prefs;
mod tab;
pub(crate) mod text;
pub(crate) mod view;
pub(crate) mod worker;

pub(crate) use checkout::GraphCheckout;
pub(crate) use error::Error;
pub(crate) use tab::GitGraph;
pub(crate) use view::{GitGraphView, GraphLook};

#[cfg(test)]
mod tests;
