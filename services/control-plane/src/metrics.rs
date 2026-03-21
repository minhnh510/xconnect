use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[derive(Default)]
struct Inner {
    auth_login_ok: AtomicU64,
    auth_login_fail: AtomicU64,
    session_created: AtomicU64,
    ws_connected: AtomicU64,
}

#[derive(Default, Clone)]
pub struct Metrics {
    inner: Arc<Inner>,
}

impl Metrics {
    pub fn inc_login_ok(&self) {
        self.inner.auth_login_ok.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_login_fail(&self) {
        self.inner.auth_login_fail.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_session_created(&self) {
        self.inner.session_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_ws_connected(&self) {
        self.inner.ws_connected.fetch_add(1, Ordering::Relaxed);
    }
}
