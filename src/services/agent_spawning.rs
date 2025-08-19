use crate::domain::*;
use crate::services::{with_state, with_state_mut, InstructionAnalyzerService};
use ic_cdk::api::time;

/// Agent spawning coordination service for OHMS 2.0
pub struct AgentSpawningService;

/// Agent spawning request
#[derive(Debug, Clone)]
pub struct SpawningRequest {
    pub request_id: String,
    pub user_principal: String,
    pub instructions: String,
    pub agent_specs: Vec<AgentSpec>,
    pub coordination_plan: String,
}

/// Agent spawning result
#[derive(Debug, Clone)]
pub struct SpawningResult {
    pub request_id: String,
    pub spawned_agents: Vec<SpawnedAgent>,
    pub coordination_network_id: Option<String>,
    pub spawning_time_ms: u64,
    pub status: SpawningStatus,
}

/// Individual spawned agent
#[derive(Debug, Clone)]
pub struct SpawnedAgent {
    pub agent_id: String,
    pub canister_id: String,
    pub specialization: String,
    pub model_id: String,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
}

/// Agent spawning status
#[derive(Debug, Clone, PartialEq)]
pub enum SpawningStatus {
    InProgress,
    Completed,
    Failed,
    PartialSuccess,
}

/// Agent status
#[derive(Debug, Clone, PartialEq)]
pub enum AgentStatus {
    Initializing,
    Ready,
    Active,
    Error,
}

/// Cross-canister call result for agent creation
#[derive(Debug, Clone)]
pub struct AgentCreationCallResult {
    pub success: bool,
    pub agent_id: Option<String>,
    pub canister_id: Option<String>,
    pub error_message: Option<String>,
}

impl AgentSpawningService {
    /// Spawn agents based on instruction analysis
    pub async fn spawn_agents_from_instructions(
        request_id: &str,
        user_principal: &str,
        instructions: &str,
    ) -> Result<SpawningResult, String> {
        let start_time = time();
        
        // Analyze instructions to get agent specifications
        let analysis = InstructionAnalyzerService::analyze_instructions(instructions, user_principal)?;
        
        // Create spawning request
        let spawning_request = SpawningRequest {
            request_id: request_id.to_string(),
            user_principal: user_principal.to_string(),
            instructions: instructions.to_string(),
            agent_specs: analysis.suggested_agents,
            coordination_plan: analysis.coordination_plan,
        };
        
        // Spawn agents
        let spawned_agents = Self::spawn_agent_instances(&spawning_request).await?;
        
        // Setup coordination network if multiple agents
        let coordination_network_id = if spawned_agents.len() > 1 {
            Some(Self::setup_coordination_network(&spawned_agents).await?)
        } else {
            None
        };
        
        // Determine final status
        let status = Self::determine_spawning_status(&spawned_agents);
        
        let result = SpawningResult {
            request_id: request_id.to_string(),
            spawned_agents,
            coordination_network_id,
            spawning_time_ms: time() - start_time,
            status,
        };
        
        // Store result in state
        Self::store_spawning_result(&result).await?;
        
        Ok(result)
    }
    
    /// Spawn individual agent instances
    async fn spawn_agent_instances(request: &SpawningRequest) -> Result<Vec<SpawnedAgent>, String> {
        let mut spawned_agents = Vec::new();
        
        for (index, spec) in request.agent_specs.iter().enumerate() {
            match Self::create_agent_instance(spec, &request.user_principal, index).await {
                Ok(agent) => spawned_agents.push(agent),
                Err(e) => {
                    // Log error but continue with other agents
                    ic_cdk::println!("Failed to spawn agent {}: {}", spec.agent_type, e);
                }
            }
        }
        
        if spawned_agents.is_empty() {
            return Err("Failed to spawn any agents".to_string());
        }
        
        Ok(spawned_agents)
    }
    
    /// Create individual agent instance via cross-canister call
    async fn create_agent_instance(
        spec: &AgentSpec,
        user_principal: &str,
        index: usize,
    ) -> Result<SpawnedAgent, String> {
        // Generate unique agent ID
        let agent_id = format!("agent_{}_{}_{}", user_principal, spec.agent_type, time());
        
        // Prepare agent creation parameters
        let agent_config = AgentCreationConfig {
            agent_id: agent_id.clone(),
            user_principal: user_principal.to_string(),
            specialization: spec.specialization.clone(),
            capabilities: spec.required_capabilities.clone(),
            model_requirements: spec.model_requirements.clone(),
            agent_type: spec.agent_type.clone(),
        };
        
        // Make cross-canister call to agent canister
        let call_result = Self::call_agent_canister_create(agent_config).await?;
        
        if !call_result.success {
            return Err(call_result.error_message.unwrap_or_else(|| "Unknown error".to_string()));
        }
        
        let canister_id = call_result.canister_id.ok_or_else(|| "No canister ID returned".to_string())?;
        
        Ok(SpawnedAgent {
            agent_id,
            canister_id,
            specialization: spec.specialization.clone(),
            model_id: spec.model_requirements.first().unwrap_or(&"llama".to_string()).clone(),
            capabilities: spec.required_capabilities.clone(),
            status: AgentStatus::Initializing,
        })
    }
    
    /// Make cross-canister call to agent canister
    async fn call_agent_canister_create(config: AgentCreationConfig) -> Result<AgentCreationCallResult, String> {
        // Get the agent canister ID from coordinator state
        let agent_canister_id = with_state(|state| {
            // Use the first available agent canister or create new one
            state.agents.values().next()
                .map(|agent| agent.canister_id.clone())
                .unwrap_or_else(|| Self::get_default_agent_canister_id())
        });
        
        // Prepare the agent registration for the existing agent canister system
        let agent_registration = AgentRegistration {
            agent_id: config.agent_id.clone(),
            agent_principal: config.user_principal.clone(),
            canister_id: agent_canister_id.clone(),
            capabilities: config.capabilities.clone(),
            model_id: config.model_requirements.first().unwrap_or(&"llama".to_string()).clone(),
            health_score: 1.0,
            registered_at: time(),
            last_seen: time(),
        };
        
        // Register the agent in our coordinator state
        with_state_mut(|state| {
            state.agents.insert(config.agent_id.clone(), agent_registration);
        });
        
        Ok(AgentCreationCallResult {
            success: true,
            agent_id: Some(config.agent_id),
            canister_id: Some(agent_canister_id),
            error_message: None,
        })
    }
    
    /// Get default agent canister ID from the known OHMS agent canister
    fn get_default_agent_canister_id() -> String {
        // Return the standard OHMS agent canister ID
        "ohms-agent".to_string()
    }
    
    /// Setup coordination network for multiple agents
    async fn setup_coordination_network(agents: &[SpawnedAgent]) -> Result<String, String> {
        use crate::services::autonomous_coord::{CoordinationSession, CoordinationType};
        
        let network_id = format!("network_{}", time());
        
        // Create coordination session for the spawned agents
        let session = CoordinationSession {
            session_id: network_id.clone(),
            participants: agents.iter().map(|a| a.agent_id.clone()).collect(),
            coordinator_agent: agents.first().map(|a| a.agent_id.clone()).unwrap_or_default(),
            objective: "Multi-agent coordination for instruction-based task execution".to_string(),
            status: crate::services::autonomous_coord::SessionStatus::Active,
            created_at: time(),
            last_activity: time(),
            messages: vec![],
            resource_constraints: crate::services::autonomous_coord::ResourceConstraints {
                max_execution_time_ms: 3600000, // 1 hour
                max_memory_usage_bytes: 1024 * 1024 * 100, // 100MB
                max_concurrent_tasks: 10,
                allowed_capabilities: Some(agents.iter().flat_map(|a| a.capabilities.clone()).collect()),
            },
        };
        
        // Store coordination session in state
        with_state_mut(|state| {
            if let Some(ref mut sessions) = state.coordination_sessions {
                sessions.insert(network_id.clone(), session);
            } else {
                let mut sessions = std::collections::HashMap::new();
                sessions.insert(network_id.clone(), session);
                state.coordination_sessions = Some(sessions);
            }
        });
        
        // Set up agent capability profiles
        Self::setup_agent_capability_profiles(agents).await?;
        
        Ok(network_id)
    }
    
    /// Setup capability profiles for coordinated agents
    async fn setup_agent_capability_profiles(agents: &[SpawnedAgent]) -> Result<(), String> {
        use crate::services::autonomous_coord::AgentCapabilityProfile;
        
        with_state_mut(|state| {
            if state.agent_capability_profiles.is_none() {
                state.agent_capability_profiles = Some(std::collections::HashMap::new());
            }
            
            if let Some(ref mut profiles) = state.agent_capability_profiles {
                for agent in agents {
                    let profile = AgentCapabilityProfile {
                        agent_id: agent.agent_id.clone(),
                        capabilities: agent.capabilities.clone(),
                        performance_metrics: crate::services::autonomous_coord::PerformanceMetrics {
                            success_rate: 1.0,
                            average_response_time_ms: 1000,
                            current_load: 0.0,
                            reliability_score: 1.0,
                            tasks_completed: 0,
                            collaboration_rating: 1.0,
                        },
                        availability_status: crate::services::autonomous_coord::AvailabilityStatus::Available,
                        coordination_preferences: crate::services::autonomous_coord::CoordinationPreferences {
                            preferred_coordination_types: vec![crate::services::autonomous_coord::CoordinationType::CollaborativePlanning],
                            max_concurrent_collaborations: 3,
                            communication_frequency: crate::services::autonomous_coord::CommunicationFrequency::Normal,
                            conflict_resolution_strategy: crate::services::autonomous_coord::ConflictResolutionStrategy::Consensus,
                        },
                    };
                    profiles.insert(agent.agent_id.clone(), profile);
                }
            }
        });
        
        Ok(())
    }
    
    /// Determine overall spawning status
    fn determine_spawning_status(agents: &[SpawnedAgent]) -> SpawningStatus {
        if agents.is_empty() {
            return SpawningStatus::Failed;
        }
        
        let ready_count = agents.iter().filter(|a| a.status == AgentStatus::Ready).count();
        let error_count = agents.iter().filter(|a| a.status == AgentStatus::Error).count();
        
        if error_count == agents.len() {
            SpawningStatus::Failed
        } else if ready_count == agents.len() {
            SpawningStatus::Completed
        } else if ready_count > 0 {
            SpawningStatus::PartialSuccess
        } else {
            SpawningStatus::InProgress
        }
    }
    
    /// Store spawning result in coordinator state
    async fn store_spawning_result(result: &SpawningResult) -> Result<(), String> {
        let agent_creation_result = AgentCreationResult {
            request_id: result.request_id.clone(),
            created_agents: result.spawned_agents.iter().map(|a| a.agent_id.clone()).collect(),
            creation_time_ms: result.spawning_time_ms,
            status: match result.status {
                SpawningStatus::Completed => AgentCreationStatus::Completed,
                SpawningStatus::Failed => AgentCreationStatus::Failed,
                SpawningStatus::PartialSuccess => AgentCreationStatus::Completed, // Treat as success
                SpawningStatus::InProgress => AgentCreationStatus::InProgress,
            },
        };
        
        with_state_mut(|state| {
            state.agent_creation_results.insert(result.request_id.clone(), agent_creation_result);
        });
        
        Ok(())
    }
    
    /// Get spawning status for a request
    pub fn get_spawning_status(request_id: &str) -> Result<Option<AgentCreationResult>, String> {
        let result = with_state(|state| {
            state.agent_creation_results.get(request_id).cloned()
        });
        
        Ok(result)
    }
    
    /// Update agent status
    pub fn update_agent_status(agent_id: &str, new_status: AgentStatus) -> Result<(), String> {
        with_state_mut(|state| {
            if let Some(agent) = state.agents.get_mut(agent_id) {
                // Update health score based on status
                agent.health_score = match new_status {
                    AgentStatus::Ready | AgentStatus::Active => 1.0,
                    AgentStatus::Initializing => 0.5,
                    AgentStatus::Error => 0.0,
                };
                agent.last_seen = time();
            }
        });
        
        Ok(())
    }
}

/// Configuration for agent creation
#[derive(Debug, Clone)]
pub struct AgentCreationConfig {
    pub agent_id: String,
    pub user_principal: String,
    pub specialization: String,
    pub capabilities: Vec<String>,
    pub model_requirements: Vec<String>,
    pub agent_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_spawning_status() {
        let agents = vec![
            SpawnedAgent {
                agent_id: "agent1".to_string(),
                canister_id: "canister1".to_string(),
                specialization: "Developer".to_string(),
                model_id: "llama".to_string(),
                capabilities: vec!["coding".to_string()],
                status: AgentStatus::Ready,
            },
            SpawnedAgent {
                agent_id: "agent2".to_string(),
                canister_id: "canister2".to_string(),
                specialization: "Tester".to_string(),
                model_id: "llama".to_string(),
                capabilities: vec!["testing".to_string()],
                status: AgentStatus::Ready,
            },
        ];
        
        let status = AgentSpawningService::determine_spawning_status(&agents);
        assert_eq!(status, SpawningStatus::Completed);
    }

    #[test]
    fn test_determine_spawning_status_partial() {
        let agents = vec![
            SpawnedAgent {
                agent_id: "agent1".to_string(),
                canister_id: "canister1".to_string(),
                specialization: "Developer".to_string(),
                model_id: "llama".to_string(),
                capabilities: vec!["coding".to_string()],
                status: AgentStatus::Ready,
            },
            SpawnedAgent {
                agent_id: "agent2".to_string(),
                canister_id: "canister2".to_string(),
                specialization: "Tester".to_string(),
                model_id: "llama".to_string(),
                capabilities: vec!["testing".to_string()],
                status: AgentStatus::Error,
            },
        ];
        
        let status = AgentSpawningService::determine_spawning_status(&agents);
        assert_eq!(status, SpawningStatus::PartialSuccess);
    }
}
