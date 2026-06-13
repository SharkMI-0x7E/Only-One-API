//! core/breaker — 熔断器（spec §4.5）

#[allow(clippy::module_inception)]
pub mod breaker;
pub mod state;

pub use breaker::Breaker;
pub use state::BreakerState;
