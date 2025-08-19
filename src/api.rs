use ic_cdk_macros::*;
use crate::domain::*;
use crate::services::{RegistryService, RoutingService, InstructionAnalyzerService, AgentSpawningService, with_state, with_state_mut};
use crate::infra::{Guards, Metrics};

#[update]
async fn register_agent(registration: AgentRegistration) -> Result<String, String> {
    Guards::require_caller_authenticated()?;
    let agent_id = RegistryService::register_agent(registration).await?;
    Metrics::increment_counter("agents_registered_total");
    Ok(agent_id)
}

#[update]
async fn route_request(request: RouteRequest) -> Result<RouteResponse, String> {
    Guards::require_caller_authenticated()?;
    Guards::validate_msg_id(&request.request_id)?;
    
    let response = RoutingService::route_request(request).await?;
    Metrics::increment_counter("requests_routed_total");
    Ok(response)
}

#[update]
async fn create_agents_from_instructions(instructions: String, agent_count: Option<u32>) -> Result<String, String> {
    Guards::require_caller_authenticated()?;
    let request_id = format!("req_{}", ic_cdk::api::time());
    let user_principal = ic_cdk::api::caller().to_string();
    
    let instruction_request = InstructionRequest {
        request_id: request_id.clone(),
        user_principal: user_principal.clone(),
        instructions: instructions.clone(),
        agent_count,
        model_preferences: vec![],
        created_at: ic_cdk::api::time(),
    };
    
    // Store instruction request
    with_state_mut(|state| {
        state.instruction_requests.insert(request_id.clone(), instruction_request);
    });
    
    // Spawn agents using the agent spawning service
    match AgentSpawningService::spawn_agents_from_instructions(&request_id, &user_principal, &instructions).await {
        Ok(_result) => {
            Metrics::increment_counter("agent_creation_requests_total");
            Ok(request_id)
        },
        Err(e) => {
            // Remove the instruction request if spawning failed
            with_state_mut(|state| {
                state.instruction_requests.remove(&request_id);
            });
            Err(format!("Failed to spawn agents: {}", e))
        }
    }
}

#[query]
fn get_agent_creation_status(request_id: String) -> Result<AgentCreationResult, String> {
    Guards::require_caller_authenticated()?;
    
    let result = with_state(|state| {
        state.agent_creation_results.get(&request_id).cloned()
    });
    
    result.ok_or_else(|| "Agent creation request not found".to_string())
}

#[query]
fn get_user_quota_status() -> Result<QuotaCheckResult, String> {
    Guards::require_caller_authenticated()?;
    let user_principal = ic_cdk::api::caller().to_string();
    
    // TODO: Implement actual quota checking with ohms-econ integration
    Ok(QuotaCheckResult {
        quota_available: true,
        remaining_agents: 10,
        monthly_limit: 25,
        tier: "Pro".to_string(),
    })
}

#[query]
fn get_agent(agent_id: String) -> Result<AgentRegistration, String> {
    Guards::require_caller_authenticated()?;
    RegistryService::get_agent(&agent_id)
}

#[query]
fn list_agents() -> Result<Vec<AgentRegistration>, String> {
    Guards::require_caller_authenticated()?;
    Ok(RegistryService::list_agents())
}

#[query]
fn list_user_agents() -> Result<Vec<AgentRegistration>, String> {
    Guards::require_caller_authenticated()?;
    let user_principal = ic_cdk::api::caller().to_string();
    
    // TODO: Filter agents by user principal
    Ok(RegistryService::list_agents())
}

#[query]
fn list_instruction_requests() -> Result<Vec<InstructionRequest>, String> {
    Guards::require_caller_authenticated()?;
    let user_principal = ic_cdk::api::caller().to_string();
    
    let requests = with_state(|state| {
        state.instruction_requests
            .values()
            .filter(|req| req.user_principal == user_principal)
            .cloned()
            .collect::<Vec<_>>()
    });
    
    Ok(requests)
}

#[query]
fn health() -> CoordinatorHealth {
    RegistryService::get_health()
}

#[query]
fn get_routing_stats(agent_id: Option<String>) -> Result<Vec<RoutingStats>, String> {
    Guards::require_caller_authenticated()?;
    Ok(RoutingService::get_stats(agent_id))
}

#[update]
fn update_agent_health(agent_id: String, health_score: f32) -> Result<(), String> {
    Guards::require_caller_authenticated()?;
    RegistryService::update_agent_health(agent_id, health_score)
}

#[update]
async fn set_swarm_policy(policy: SwarmPolicy) -> Result<(), String> {
    Guards::require_caller_authenticated()?;
    with_state_mut(|s| { s.config.swarm = policy; });
    Ok(())
}

#[query]
fn get_swarm_policy() -> SwarmPolicy {
    with_state(|s| s.config.swarm.clone())
}

#[update]
async fn route_best_result(request: RouteRequest, top_k: u32, window_ms: u64) -> Result<RouteResponse, String> {
    Guards::require_caller_authenticated()?;
    Guards::validate_msg_id(&request.request_id)?;
    RoutingService::fanout_best_result(request, top_k as usize, window_ms).await
}

#[query]
fn get_instruction_analysis(request_id: String) -> Result<InstructionAnalysisResult, String> {
    Guards::require_caller_authenticated()?;
    
    // Get the instruction request
    let instruction_request = with_state(|state| {
        state.instruction_requests.get(&request_id).cloned()
    });
    
    let instruction_request = instruction_request.ok_or_else(|| "Instruction request not found".to_string())?;
    
    // Analyze the instructions
    InstructionAnalyzerService::analyze_instructions(&instruction_request.instructions, &instruction_request.user_principal)
}