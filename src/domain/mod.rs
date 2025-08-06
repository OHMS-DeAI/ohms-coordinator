use serde::{Deserialize, Serialize};
use candid::CandidType;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct AgentRegistration {
    pub agent_id: String,
    pub agent_principal: String,
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
    Competition,  // Open bounty competition
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct RouteResponse {
    pub request_id: String,
    pub selected_agents: Vec<String>,
    pub routing_time_ms: u64,
    pub selection_criteria: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct BountySpec {
    pub title: String,
    pub description: String,
    pub required_capabilities: Vec<String>,
    pub max_participants: u32,
    pub deadline_timestamp: u64,
    pub escrow_amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct Bounty {
    pub bounty_id: String,
    pub spec: BountySpec,
    pub creator: String,
    pub escrow_id: String,
    pub status: BountyStatus,
    pub created_at: u64,
    pub submissions: Vec<BountySubmission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum BountyStatus {
    Open,
    InProgress,
    Resolved,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct BountySubmission {
    pub submission_id: String,
    pub bounty_id: String,
    pub agent_id: String,
    pub payload: Vec<u8>,
    pub submitted_at: u64,
    pub evaluation_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct BountyResolution {
    pub bounty_id: String,
    pub winner_id: Option<String>,
    pub resolution_type: ResolutionType,
    pub resolved_at: u64,
    pub settlement_details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum ResolutionType {
    WinnerSelected,
    NoWinner,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CoordinatorHealth {
    pub total_agents: u32,
    pub active_agents: u32,
    pub total_bounties: u32,
    pub active_bounties: u32,
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