//! service/admin — Admin API (spec §5.7)
//!
//! Separate listener (default 127.0.0.1:9090), must not be exposed to the public internet.

pub mod auth;
pub mod routes;

use std::sync::Arc;

use axum::Router;

use crate::service::state::AppState;

/// Build the admin API router
pub fn admin_router(state: Arc<AppState>) -> Router {
    routes::admin_routes(state)
}
