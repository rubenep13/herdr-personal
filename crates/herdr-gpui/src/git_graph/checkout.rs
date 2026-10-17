//! The checkout a Git graph tab shows, on the tab's endpoint, which may be an
//! SSH host. Saved with the tabs, so it is checked again whenever it is read.

use super::error::{Error, Result};
use crate::browser::{checkout_name, checkout_path_ok};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SavedGraphCheckout")]
pub(crate) struct GraphCheckout {
    pub(crate) checkout: String,
}

#[derive(Deserialize)]
struct SavedGraphCheckout {
    checkout: String,
}

impl TryFrom<SavedGraphCheckout> for GraphCheckout {
    type Error = Error;

    fn try_from(saved: SavedGraphCheckout) -> Result<Self> {
        Self::new(saved.checkout)
    }
}

impl GraphCheckout {
    pub(crate) fn new(checkout: String) -> Result<Self> {
        if !checkout_path_ok(&checkout) {
            return Err(Error::InvalidCheckout);
        }
        Ok(Self { checkout })
    }

    /// The checkout's folder name.
    pub(crate) fn name(&self) -> &str {
        checkout_name(&self.checkout)
    }
}
