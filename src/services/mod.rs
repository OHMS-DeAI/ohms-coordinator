use crate::domain::*;
use ic_cdk::api::time;
use std::collections::HashMap;
use std::cell::RefCell;

pub mod registry;
pub mod routing;
pub mod dedup;
pub mod quota_manager;
pub mod autonomous_coord;
pub mod instruction_analyzer;
pub mod agent_spawning;

pub use registry::RegistryService;
pub use routing::RoutingService;
pub use dedup::DedupService;
pub use quota_manager::QuotaManager;
pub use autonomous_coord::AutonomousCoordinationService;
pub use instruction_analyzer::InstructionAnalyzerService;
pub use agent_spawning::AgentSpawningService;

thread_local! {
    static STATE: RefCell<CoordinatorState> = RefCell::new(CoordinatorState::default());
}

#[derive(Debug, Default)]
pub struct CoordinatorState {
    pub agents: HashMap<String, AgentRegistration>,
    pub instruction_requests: HashMap<String, InstructionRequest>,
    pub agent_creation_results: HashMap<String, AgentCreationResult>,
    pub dedup_cache: HashMap<String, DedupEntry>,
    pub routing_stats: HashMap<String, RoutingStats>,
    pub user_quotas: HashMap<String, quota_manager::UserQuota>,
    pub metrics: CoordinatorMetrics,
    pub config: CoordinatorConfig,
    // Autonomous coordination fields
    pub coordination_sessions: Option<HashMap<String, autonomous_coord::CoordinationSession>>,
    pub agent_capability_profiles: Option<HashMap<String, autonomous_coord::AgentCapabilityProfile>>,
    pub agent_message_queues: Option<HashMap<String, Vec<autonomous_coord::AgentMessage>>>,
}

#[derive(Debug, Default)]
pub struct CoordinatorMetrics {
    pub total_routes: u64,
    pub total_agent_creations: u64,
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