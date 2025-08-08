use crate::domain::*;
use ic_cdk::api::time;
use std::collections::HashMap;
use std::cell::RefCell;

pub mod registry;
pub mod routing;
pub mod bounty;
pub mod dedup;

pub use registry::RegistryService;
pub use routing::RoutingService;
pub use bounty::BountyService;
pub use dedup::DedupService;

thread_local! {
    static STATE: RefCell<CoordinatorState> = RefCell::new(CoordinatorState::default());
}

#[derive(Debug, Default)]
pub struct CoordinatorState {
    pub agents: HashMap<String, AgentRegistration>,
    pub bounties: HashMap<String, Bounty>,
    pub dedup_cache: HashMap<String, DedupEntry>,
    pub routing_stats: HashMap<String, RoutingStats>,
    pub metrics: CoordinatorMetrics,
    pub config: CoordinatorConfig,
}

#[derive(Debug, Default)]
pub struct CoordinatorMetrics {
    pub total_routes: u64,
    pub total_bounties: u64,
    pub total_agents: u64,
    pub average_routing_time_ms: f64,
    pub last_activity: u64,
}

pub fn with_state<R>(f: impl FnOnce(&CoordinatorState) -> R) -> R {
    STATE.with(|s| f(&*s.borrow()))
}

pub fn with_state_mut<R>(f: impl FnOnce(&mut CoordinatorState) -> R) -> R {
    STATE.with(|s| f(&mut *s.borrow_mut()))
}