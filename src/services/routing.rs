use crate::domain::*;
use crate::services::{with_state, with_state_mut, RegistryService, DedupService};
use ic_cdk::api::time;
use rand::{SeedableRng, Rng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

pub struct RoutingService;

impl RoutingService {
    pub async fn route_request(request: RouteRequest) -> Result<RouteResponse, String> {
        let start_time = time();
        
        // Check for duplicate request
        if DedupService::is_duplicate(&request.request_id) {
            return Err("Duplicate request ID".to_string());
        }
        
        let selected_agents = match request.routing_mode {
            RoutingMode::Unicast => Self::select_best_agent(&request.capabilities_required)?,
            RoutingMode::Broadcast => Self::select_multiple_agents(&request.capabilities_required, 3)?,
            RoutingMode::Competition => Self::select_competitive_agents(&request.capabilities_required, 5)?,
        };
        
        let routing_time_ms = time() - start_time;
        
        let response = RouteResponse {
            request_id: request.request_id.clone(),
            selected_agents: selected_agents.iter().map(|a| a.agent_id.clone()).collect(),
            routing_time_ms,
            selection_criteria: format!("Selected by {:?} routing", request.routing_mode),
        };
        
        // Record the routing decision in dedup cache
        DedupService::record_request(&request.request_id, &response)?;
        
        // Update metrics
        with_state_mut(|state| {
            state.metrics.total_routes += 1;
            let new_avg = (state.metrics.average_routing_time_ms * (state.metrics.total_routes - 1) as f64 
                + routing_time_ms as f64) / state.metrics.total_routes as f64;
            state.metrics.average_routing_time_ms = new_avg;
            state.metrics.last_activity = time();
        });
        
        Ok(response)
    }
    
    fn select_best_agent(capabilities: &[String]) -> Result<Vec<AgentRegistration>, String> {
        let candidates = Self::get_capable_agents(capabilities);
        if candidates.is_empty() {
            return Err("No agents available with required capabilities".to_string());
        }
        
        // Select agent with best health * capability fit score
        let best = candidates
            .into_iter()
            .max_by(|a, b| {
                let score_a = Self::calculate_agent_score(a, capabilities);
                let score_b = Self::calculate_agent_score(b, capabilities);
                score_a.partial_cmp(&score_b).unwrap()
            })
            .unwrap();
        
        Ok(vec![best])
    }
    
    fn select_multiple_agents(capabilities: &[String], k: usize) -> Result<Vec<AgentRegistration>, String> {
        let mut candidates = Self::get_capable_agents(capabilities);
        if candidates.is_empty() {
            return Err("No agents available with required capabilities".to_string());
        }
        
        // Sort by score and take top K
        candidates.sort_by(|a, b| {
            let score_a = Self::calculate_agent_score(a, capabilities);
            let score_b = Self::calculate_agent_score(b, capabilities);
            score_b.partial_cmp(&score_a).unwrap() // Descending order
        });
        
        candidates.truncate(k);
        Ok(candidates)
    }
    
    fn select_competitive_agents(capabilities: &[String], max_agents: usize) -> Result<Vec<AgentRegistration>, String> {
        let candidates = Self::get_capable_agents(capabilities);
        if candidates.is_empty() {
            return Err("No agents available for competition".to_string());
        }
        
        // For competition mode, include diversity - not just top scorers
        let mut selected = Vec::new();
        let mut rng = ChaCha8Rng::seed_from_u64(time());
        
        for agent in candidates.into_iter().take(max_agents) {
            if agent.health_score > 0.3 && rng.gen::<f32>() > 0.2 {
                selected.push(agent);
            }
        }
        
        Ok(selected)
    }
    
    fn get_capable_agents(capabilities: &[String]) -> Vec<AgentRegistration> {
        let healthy_agents = RegistryService::get_healthy_agents(0.1);
        healthy_agents
            .into_iter()
            .filter(|agent| {
                capabilities.iter().any(|cap| agent.capabilities.contains(cap))
            })
            .collect()
    }
    
    fn calculate_agent_score(agent: &AgentRegistration, required_capabilities: &[String]) -> f32 {
        let health_weight = 0.6;
        let capability_weight = 0.4;
        
        let health_score = agent.health_score;
        
        let capability_score = required_capabilities
            .iter()
            .map(|cap| {
                if agent.capabilities.contains(cap) { 1.0 } else { 0.0 }
            })
            .sum::<f32>() / required_capabilities.len().max(1) as f32;
        
        health_weight * health_score + capability_weight * capability_score
    }
    
    pub fn get_stats(agent_id: Option<String>) -> Vec<RoutingStats> {
        with_state(|state| {
            match agent_id {
                Some(id) => state.routing_stats.get(&id).cloned().into_iter().collect(),
                None => state.routing_stats.values().cloned().collect(),
            }
        })
    }
    
    pub fn update_agent_stats(agent_id: &str, success: bool, response_time_ms: u64) {
        with_state_mut(|state| {
            if let Some(stats) = state.routing_stats.get_mut(agent_id) {
                stats.total_requests += 1;
                
                let old_success_rate = stats.success_rate;
                let old_total = (stats.total_requests - 1) as f32;
                let new_success_rate = if success {
                    (old_success_rate * old_total + 1.0) / stats.total_requests as f32
                } else {
                    (old_success_rate * old_total) / stats.total_requests as f32
                };
                stats.success_rate = new_success_rate;
                
                let new_avg_time = (stats.average_response_time_ms * old_total as f64 
                    + response_time_ms as f64) / stats.total_requests as f64;
                stats.average_response_time_ms = new_avg_time;
            }
        });
    }
}