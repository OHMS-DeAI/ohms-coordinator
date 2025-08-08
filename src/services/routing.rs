use crate::domain::*;
use crate::services::{with_state, with_state_mut, RegistryService, DedupService};
use ic_cdk::api::time;
use candid::{Principal, CandidType};
use serde::Deserialize;
use ic_cdk::api::call::call;
use futures::future::join_all;
use sha2::{Sha256, Digest};

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
        
        // Optionally trigger downstream calls (not returning results here; response carries selection)
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
        
        // For competition mode, include top scored agents up to max_agents
        let mut pool = candidates;
        pool.sort_by(|a, b| {
            let score_a = Self::calculate_agent_score(a, capabilities);
            let score_b = Self::calculate_agent_score(b, capabilities);
            score_b.partial_cmp(&score_a).unwrap()
        });
        let selected: Vec<AgentRegistration> = pool.into_iter().take(max_agents).collect();
        
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

    pub async fn fanout_best_result(request: RouteRequest, k: usize, window_ms: u64) -> Result<RouteResponse, String> {
        // Enforce subscription tier cap (temporary: cap to 3)
        let cap_k = k.min(3);
        let agents = Self::select_multiple_agents(&request.capabilities_required, cap_k)?;
        if agents.is_empty() { return Err("No agents available".to_string()); }

        let start = time();

        // Build prompt and request payload for agents
        let prompt = String::from_utf8(request.payload.clone()).unwrap_or_else(|_| "".to_string());
        let seed = Self::derive_seed(&request.request_id);
        let msg_id = request.request_id.clone();

        // Dispatch concurrent calls
        let futures = agents.iter().map(|agent| {
            let canister_id = agent.canister_id.clone();
            let agent_id = agent.agent_id.clone();
            let req = AInferenceRequest::new(seed, &prompt, &msg_id);
            async move {
                let started = time();
                let pr = Principal::from_text(canister_id.clone())
                    .map_err(|e| format!("Invalid canister id for agent {}: {}", agent_id, e))?;
                // Call agent.infer(InferenceRequest)
                let (result,): (AResult2,) = call(pr, "infer", (req,)).await
                    .map_err(|e| format!("infer call failed for {}: {:?}", agent_id, e))?;
                let elapsed = time() - started;

                let scored = match result {
                    AResult2::Ok(resp) => {
                        // Run lightweight verifiers
                        let evidence = Self::run_verifiers(&resp);
                        let score = Self::score_response(&resp, elapsed) + if evidence.passed { 0.1 } else { 0.0 };
                        Ok((agent_id, elapsed, Some(resp), score))
                    },
                    AResult2::Err(err) => Err(format!("agent {} error: {}", agent_id, err)),
                };
                scored
            }
        });

        let results = join_all(futures).await;

        // Choose best among those within window
        let mut best_agent: Option<(String, u64, f32)> = None; // (agent_id, elapsed, score)
        let mut selected_ids: Vec<String> = Vec::new();
        for res in results.into_iter() {
            match res {
                Ok((agent_id, elapsed, _resp_opt, score)) => {
                    selected_ids.push(agent_id.clone());
                    if elapsed <= window_ms {
                        if let Some((_, _, best_score)) = &best_agent {
                            if score > *best_score {
                                best_agent = Some((agent_id.clone(), elapsed, score));
                            }
                        } else {
                            best_agent = Some((agent_id.clone(), elapsed, score));
                        }
                    }
                }
                Err(_e) => {
                    // Skip failed agent
                    continue;
                }
            }
        }

        // Winner prioritization: put winner first if exists
        if let Some((winner_id, _elapsed, _score)) = &best_agent {
            selected_ids.sort_by_key(|id| if id == winner_id { 0 } else { 1 });
        }

        let resp = RouteResponse {
            request_id: request.request_id.clone(),
            selected_agents: selected_ids,
            routing_time_ms: time() - start,
            selection_criteria: format!("fanout_top_k={} window_ms={} winner={}", cap_k, window_ms, best_agent.as_ref().map(|(w,_,_)| w.clone()).unwrap_or_default()),
        };
        DedupService::record_request(&request.request_id, &resp)?;
        Ok(resp)
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

// Local mirror types to call ohms-agent canister
#[derive(Clone, Debug, CandidType, Deserialize)]
struct ADecodeParams {
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    repetition_penalty: Option<f32>,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
struct AInferenceRequest {
    seed: u64,
    prompt: String,
    decode_params: ADecodeParams,
    msg_id: String,
}

impl AInferenceRequest {
    fn new(seed: u64, prompt: &str, msg_id: &str) -> Self {
        Self {
            seed,
            prompt: prompt.to_string(),
            decode_params: ADecodeParams { max_tokens: Some(128), temperature: Some(0.7), top_p: Some(0.9), top_k: None, repetition_penalty: None },
            msg_id: msg_id.to_string(),
        }
    }
}

#[derive(Clone, Debug, CandidType, Deserialize)]
struct AInferenceResponse {
    tokens: Vec<String>,
    generated_text: String,
    inference_time_ms: u64,
    cache_hits: u32,
    cache_misses: u32,
}

#[derive(Clone, Debug, CandidType, Deserialize)]
enum AResult2 {
    Ok(AInferenceResponse),
    Err(String),
}

impl RoutingService {
    fn derive_seed(msg_id: &str) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(msg_id.as_bytes());
        let digest = hasher.finalize();
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&digest[..8]);
        u64::from_be_bytes(bytes)
    }

    fn score_response(resp: &AInferenceResponse, elapsed_ms: u64) -> f32 {
        // Simple heuristic: positive credit for content length and tokens count; negative for latency
        let len_score = (resp.generated_text.len() as f32).min(1000.0) / 1000.0; // cap
        let tok_score = (resp.tokens.len() as f32).min(256.0) / 256.0;
        let latency_penalty = (elapsed_ms as f32) / 5000.0; // 5s baseline
        let cache_bonus = if resp.cache_hits + resp.cache_misses > 0 { (resp.cache_hits as f32) / ((resp.cache_hits + resp.cache_misses) as f32) * 0.1 } else { 0.0 };
        (0.6 * len_score) + (0.3 * tok_score) + cache_bonus - (0.4 * latency_penalty)
    }

    fn run_verifiers(resp: &AInferenceResponse) -> VerifierEvidence {
        // Simple validators: ensure non-empty, attempt JSON parse if starts with '{'
        if resp.generated_text.trim().is_empty() {
            return VerifierEvidence { passed: false, details: "empty output".to_string() };
        }
        if resp.generated_text.trim_start().starts_with('{') {
            // shallow JSON key check for demo
            let has_colon = resp.generated_text.contains(':');
            if !has_colon {
                return VerifierEvidence { passed: false, details: "invalid json shape".to_string() };
            }
        }
        VerifierEvidence { passed: true, details: "basic checks pass".to_string() }
    }
}