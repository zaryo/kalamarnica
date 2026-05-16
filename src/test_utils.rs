use std::sync::LazyLock;
use std::sync::Mutex;

use crate::context::Context;
use crate::transport::Transport;

pub static CWD_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[must_use]
pub fn sample_context() -> Context {
    Context {
        hostname: "github.com".to_owned(),
        user: "testuser".to_owned(),
        transport: Transport::Ssh,
    }
}

#[must_use]
pub fn sample_gitlab_context() -> Context {
    Context {
        hostname: "gitlab.com".to_owned(),
        user: "testuser".to_owned(),
        transport: Transport::Ssh,
    }
}
