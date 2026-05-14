#[cfg(feature = "ui")]
use axum::Router;
#[cfg(feature = "ui")]
use std::sync::Arc;
#[cfg(feature = "ui")]
use backyard_core::Queue;

#[cfg(feature = "ui")]
pub fn router(_queue: Arc<dyn Queue>) -> Router {
    Router::new()
}

#[cfg(not(feature = "ui"))]
pub fn router(_queue: Arc<dyn Queue>) -> () {
    ()
}
