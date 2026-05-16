use serde::Deserialize;
use serde::Serialize;

use crate::transport::Transport;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Context {
    pub hostname: String,
    pub user: String,
    pub transport: Transport,
}
