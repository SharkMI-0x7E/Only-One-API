//! core/routing — 路由匹配引擎（spec §4.3）

pub mod canary;
pub mod matcher;
pub mod table;

pub use canary::CanaryRouter;
pub use matcher::Matcher;
pub use table::{RouteTable, Router};
