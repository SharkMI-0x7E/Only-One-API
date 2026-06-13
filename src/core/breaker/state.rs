//! 熔断器状态机（spec §4.5）

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    /// 正常通过
    Closed,
    /// 熔断打开，请求直接拒绝
    Open,
    /// 半开：放行一个探测请求
    HalfOpen,
}
