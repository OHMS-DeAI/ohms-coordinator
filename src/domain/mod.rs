use serde::{Deserialize, Serialize};
use candid::CandidType;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct AgentRegistration {
    pub agent_id: String,
    pub agent_principal: String,
    pub canister_id: String,
    pub capabilities: Vec<String>,
    pub model_id: String,
    pub health_score: f32,
    pub registered_at: u64,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct RouteRequest {
    pub request_id: String,
    pub requester: String,
    pub capabilities_required: Vec<String>,
    pub payload: Vec<u8>,
    pub routing_mode: RoutingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum RoutingMode {
    Unicast,      // Route to single best agent
    Broadcast,    // Route to multiple agents (K agents)
    AgentSpawning, // Agent creation coordination
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct RouteResponse {
    pub request_id: String,
    pub selected_agents: Vec<String>,
    pub routing_time_ms: u64,
    pub selection_criteria: String,
}

// OHMS 2.0: Agent creation and instruction processing types
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct InstructionRequest {
    pub request_id: String,
    pub user_principal: String,
    pub instructions: String,
    pub agent_count: Option<u32>,
    pub model_preferences: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct AgentCreationResult {
    pub request_id: String,
    pub created_agents: Vec<String>,
    pub creation_time_ms: u64,
    pub status: AgentCreationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType, PartialEq, Copy)]
pub enum AgentCreationStatus {
    InProgress,
    Completed,
    Failed,
    QuotaExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CoordinatorHealth {
    pub total_agents: u32,
    pub active_agents: u32,
    pub total_agent_creations: u32,
    pub active_instructions: u32,
    pub total_routes_processed: u64,
    pub average_routing_time_ms: f64,
    pub dedup_cache_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct RoutingStats {
    pub agent_id: String,
    pub total_requests: u64,
    pub success_rate: f32,
    pub average_response_time_ms: f64,
    pub capability_scores: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupEntry {
    pub msg_id: String,
    pub processed_at: u64,
    pub result_hash: String,
    pub ttl_expires_at: u64,
}

// Swarm/Hive policy
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum SwarmTopology { Mesh, Hierarchical, Ring, Star }

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum OrchestrationMode { Parallel, Sequential, Adaptive }

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct SwarmPolicy {
    pub topology: SwarmTopology,
    pub mode: OrchestrationMode,
    pub top_k: u32,
    pub window_ms: u64,
}

impl Default for SwarmPolicy {
    fn default() -> Self {
        Self { topology: SwarmTopology::Mesh, mode: OrchestrationMode::Parallel, top_k: 3, window_ms: 100 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CoordinatorConfig {
    pub swarm: SwarmPolicy,
}

impl Default for CoordinatorConfig {
    fn default() -> Self { Self { swarm: SwarmPolicy::default() } }
}

// OHMS 2.0: Agent spawning and coordination types
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct AgentSpawningRequest {
    pub request_id: String,
    pub user_principal: String,
    pub instructions: String,
    pub agent_specifications: Vec<AgentSpec>,
    pub coordination_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct AgentSpec {
    pub agent_type: String,
    pub required_capabilities: Vec<String>,
    pub model_requirements: Vec<String>,
    pub specialization: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct InstructionAnalysisResult {
    pub request_id: String,
    pub parsed_requirements: Vec<String>,
    pub suggested_agents: Vec<AgentSpec>,
    pub coordination_plan: String,
    pub quota_check: QuotaCheckResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct QuotaCheckResult {
    pub quota_available: bool,
    pub remaining_agents: u32,
    pub monthly_limit: u32,
    pub tier: String,
}

// Simple validation types for routing service
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct VerifierEvidence {
    pub passed: bool,
    pub details: String,
}