use ic_cdk_macros::*;
use crate::domain::*;
use crate::services::{RegistryService, RoutingService, BountyService};
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
async fn open_bounty(spec: BountySpec, escrow_id: String) -> Result<String, String> {
    Guards::require_caller_authenticated()?;
    let bounty_id = BountyService::open_bounty(spec, escrow_id).await?;
    Metrics::increment_counter("bounties_opened_total");
    Ok(bounty_id)
}

#[update]
async fn submit_result(bounty_id: String, agent_id: String, payload: Vec<u8>) -> Result<String, String> {
    Guards::require_caller_authenticated()?;
    let submission_id = BountyService::submit_result(bounty_id, agent_id, payload).await?;
    Metrics::increment_counter("bounty_submissions_total");
    Ok(submission_id)
}

#[update]
async fn resolve_bounty(bounty_id: String, winner_id: Option<String>) -> Result<BountyResolution, String> {
    Guards::require_caller_authenticated()?;
    let resolution = BountyService::resolve_bounty(bounty_id, winner_id).await?;
    Metrics::increment_counter("bounties_resolved_total");
    Ok(resolution)
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
fn get_bounty(bounty_id: String) -> Result<Bounty, String> {
    Guards::require_caller_authenticated()?;
    BountyService::get_bounty(&bounty_id)
}

#[query]
fn list_bounties() -> Result<Vec<Bounty>, String> {
    Guards::require_caller_authenticated()?;
    Ok(BountyService::list_bounties())
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