use crate::domain::*;
use crate::services::{with_state, with_state_mut};
use ic_cdk::api::time;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

pub struct RegistryService;

impl RegistryService {
    pub async fn register_agent(registration: AgentRegistration) -> Result<String, String> {
        let now = time();
        let agent_id = Self::generate_agent_id(&registration.agent_principal, &registration.model_id);
        
        let mut agent_reg = registration;
        agent_reg.agent_id = agent_id.clone();
        agent_reg.registered_at = now;
        agent_reg.last_seen = now;
        agent_reg.health_score = 1.0; // Start with perfect health
        
        with_state_mut(|state| {
            state.agents.insert(agent_id.clone(), agent_reg.clone());
            
            // Initialize routing stats for this agent
            let stats = RoutingStats {
                agent_id: agent_id.clone(),
                total_requests: 0,
                success_rate: 1.0,
                average_response_time_ms: 0.0,
                capability_scores: agent_reg.capabilities
                    .iter()
                    .map(|cap| (cap.clone(), 1.0))
                    .collect(),
            };
            state.routing_stats.insert(agent_id.clone(), stats);
            
            state.metrics.total_agents += 1;
            state.metrics.last_activity = now;
        });
        
        Ok(agent_id)
    }
    
    pub fn get_agent(agent_id: &str) -> Result<AgentRegistration, String> {
        with_state(|state| {
            state.agents
                .get(agent_id)
                .cloned()
                .ok_or_else(|| format!("Agent not found: {}", agent_id))
        })
    }
    
    pub fn list_agents() -> Vec<AgentRegistration> {
        with_state(|state| state.agents.values().cloned().collect())
    }
    
    pub fn update_agent_health(agent_id: String, health_score: f32) -> Result<(), String> {
        let now = time();
        let clamped_score = health_score.max(0.0).min(1.0);
        
        with_state_mut(|state| {
            if let Some(agent) = state.agents.get_mut(&agent_id) {
                agent.health_score = clamped_score;
                agent.last_seen = now;
                Ok(())
            } else {
                Err(format!("Agent not found: {}", agent_id))
            }
        })
    }
    
    pub fn get_agents_by_capability(capability: &str) -> Vec<AgentRegistration> {
        with_state(|state| {
            state.agents
                .values()
                .filter(|agent| agent.capabilities.contains(&capability.to_string()))
                .cloned()
                .collect()
        })
    }
    
    pub fn get_healthy_agents(min_health: f32) -> Vec<AgentRegistration> {
        with_state(|state| {
            state.agents
                .values()
                .filter(|agent| agent.health_score >= min_health)
                .cloned()
                .collect()
        })
    }
    
    pub fn get_health() -> CoordinatorHealth {
        with_state(|state| {
            let total_agents = state.agents.len() as u32;
            let active_agents = state.agents
                .values()
                .filter(|agent| agent.health_score > 0.5)
                .count() as u32;
            
            let total_agent_creations = state.agent_creation_results.len() as u32;
            let active_instructions = state.instruction_requests
                .values()
                .count() as u32;
            
            CoordinatorHealth {
                total_agents,
                active_agents,
                total_agent_creations,
                active_instructions,
                total_routes_processed: state.metrics.total_routes,
                average_routing_time_ms: state.metrics.average_routing_time_ms,
                dedup_cache_size: state.dedup_cache.len() as u32,
            }
        })
    }
    
    fn generate_agent_id(principal: &str, model_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(principal.as_bytes());
        hasher.update(model_id.as_bytes());
        hasher.update(time().to_be_bytes());
        let hash = hasher.finalize();
        format!("agent_{}", general_purpose::STANDARD.encode(&hash[..8]))
    }
}