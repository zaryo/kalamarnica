use serde::Deserialize;
use serde::Serialize;

use crate::vcs::Vcs;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyContext {
    pub name: String,
    pub vcs: Vcs,
}
