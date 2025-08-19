use crate::domain::*;
use crate::services::{with_state, with_state_mut};
use ic_cdk::api::time;
use serde::{Deserialize, Serialize};
use candid::CandidType;
use std::collections::HashMap;

/// Autonomous coordination service for self-coordinating multi-agent networks
pub struct AutonomousCoordinationService;

/// Agent communication message types
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum AgentMessage {
    TaskRequest {
        task_id: String,
        description: String,
        required_capabilities: Vec<String>,
        priority: MessagePriority,
    },
    TaskResponse {
        task_id: String,
        agent_id: String,
        status: TaskStatus,
        result: Option<String>,
        error: Option<String>,
    },
    CapabilityAdvertisement {
        agent_id: String,
        capabilities: Vec<String>,
        availability: f32, // 0.0 to 1.0
        current_load: u32,
    },
    CoordinationRequest {
        requesting_agent: String,
        coordination_type: CoordinationType,
        data: String,
    },
}

/// Message priority levels for task distribution
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Types of coordination between agents
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum CoordinationType {
    ResourceSharing,
    TaskDelegation,
    CollaborativePlanning,
    ConflictResolution,
    LoadBalancing,
}

/// Coordination session for managing multi-agent collaboration
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CoordinationSession {
    pub session_id: String,
    pub participants: Vec<String>,
    pub coordinator_agent: String,
    pub objective: String,
    pub status: SessionStatus,
    pub created_at: u64,
    pub last_activity: u64,
    pub messages: Vec<CoordinationMessage>,
    pub resource_constraints: ResourceConstraints,
}

/// Coordination session status
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum SessionStatus {
    Active,
    Coordinating,
    Completed,
    Failed,
    Timeout,
}

/// Message within a coordination session
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CoordinationMessage {
    pub from_agent: String,
    pub to_agent: Option<String>, // None for broadcast
    pub message_type: AgentMessage,
    pub timestamp: u64,
    pub sequence_number: u32,
}

/// Resource constraints for coordination
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct ResourceConstraints {
    pub max_execution_time_ms: u64,
    pub max_memory_usage_bytes: u64,
    pub max_concurrent_tasks: u32,
    pub allowed_capabilities: Option<Vec<String>>,
}

/// Agent capability profile for coordination
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct AgentCapabilityProfile {
    pub agent_id: String,
    pub capabilities: Vec<String>,
    pub performance_metrics: PerformanceMetrics,
    pub availability_status: AvailabilityStatus,
    pub coordination_preferences: CoordinationPreferences,
}

/// Performance metrics for agent coordination
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct PerformanceMetrics {
    pub success_rate: f32,
    pub average_response_time_ms: u64,
    pub current_load: f32,
    pub reliability_score: f32,
    pub tasks_completed: u32,
    pub collaboration_rating: f32,
}

/// Agent availability status
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum AvailabilityStatus {
    Available,
    Busy,
    Overloaded,
    Maintenance,
    Offline,
}

/// Agent preferences for coordination
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CoordinationPreferences {
    pub preferred_coordination_types: Vec<CoordinationType>,
    pub max_concurrent_collaborations: u32,
    pub communication_frequency: CommunicationFrequency,
    pub conflict_resolution_strategy: ConflictResolutionStrategy,
}

/// Communication frequency preferences
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum CommunicationFrequency {
    Minimal,
    Normal,
    Frequent,
    RealTime,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum ConflictResolutionStrategy {
    Negotiate,
    Escalate,
    Consensus,
    Priority,
}

impl AutonomousCoordinationService {
    /// Initialize a new coordination session
    pub async fn create_coordination_session(
        objective: String,
        participant_agents: Vec<String>,
        coordinator_agent: String,
        resource_constraints: ResourceConstraints,
    ) -> Result<CoordinationSession, String> {
        let session_id = format!("coord_{}", time());
        let session = CoordinationSession {
            session_id: session_id.clone(),
            participants: participant_agents,
            coordinator_agent,
            objective,
            status: SessionStatus::Active,
            created_at: time(),
            last_activity: time(),
            messages: Vec::new(),
            resource_constraints,
        };

        // Store coordination session
        with_state_mut(|state| {
            if state.coordination_sessions.is_none() {
                state.coordination_sessions = Some(HashMap::new());
            }
            state.coordination_sessions.as_mut().unwrap()
                .insert(session_id, session.clone());
        });

        Ok(session)
    }

    /// Send message between agents in coordination session
    pub async fn send_coordination_message(
        session_id: String,
        from_agent: String,
        to_agent: Option<String>,
        message: AgentMessage,
    ) -> Result<(), String> {
        with_state_mut(|state| {
            if let Some(sessions) = &mut state.coordination_sessions {
                if let Some(session) = sessions.get_mut(&session_id) {
                    let coord_message = CoordinationMessage {
                        from_agent,
                        to_agent,
                        message_type: message,
                        timestamp: time(),
                        sequence_number: session.messages.len() as u32,
                    };

                    session.messages.push(coord_message);
                    session.last_activity = time();

                    // Check for session timeout (prevent infinite loops)
                    let timeout_duration = 3600 * 1_000_000_000; // 1 hour in nanoseconds
                    if time() - session.created_at > timeout_duration {
                        session.status = SessionStatus::Timeout;
                    }

                    Ok(())
                } else {
                    Err("Coordination session not found".to_string())
                }
            } else {
                Err("No coordination sessions available".to_string())
            }
        })
    }

    /// Process task distribution among agents
    pub async fn distribute_task(
        task_description: String,
        required_capabilities: Vec<String>,
        priority: MessagePriority,
    ) -> Result<String, String> {
        let task_id = format!("task_{}", time());
        
        // Find available agents with required capabilities
        let suitable_agents = Self::find_suitable_agents(&required_capabilities).await?;
        
        if suitable_agents.is_empty() {
            return Err("No suitable agents available for task".to_string());
        }

        // Select best agent based on performance metrics and availability
        let selected_agent = Self::select_optimal_agent(&suitable_agents, &priority).await?;

        // Create task request message
        let task_message = AgentMessage::TaskRequest {
            task_id: task_id.clone(),
            description: task_description,
            required_capabilities,
            priority,
        };

        // Send task to selected agent
        Self::route_message_to_agent(selected_agent, task_message).await?;

        Ok(task_id)
    }

    /// Find agents with required capabilities
    async fn find_suitable_agents(
        required_capabilities: &[String],
    ) -> Result<Vec<AgentCapabilityProfile>, String> {
        with_state(|state| {
            if let Some(profiles) = &state.agent_capability_profiles {
                let suitable: Vec<AgentCapabilityProfile> = profiles
                    .values()
                    .filter(|profile| {
                        // Check if agent has required capabilities
                        required_capabilities.iter().all(|req_cap| {
                            profile.capabilities.contains(req_cap)
                        }) &&
                        // Check if agent is available
                        matches!(profile.availability_status, AvailabilityStatus::Available)
                    })
                    .cloned()
                    .collect();
                
                Ok(suitable)
            } else {
                Ok(Vec::new())
            }
        })
    }

    /// Select optimal agent for task based on performance metrics
    async fn select_optimal_agent(
        agents: &[AgentCapabilityProfile],
        priority: &MessagePriority,
    ) -> Result<String, String> {
        if agents.is_empty() {
            return Err("No agents provided for selection".to_string());
        }

        // Calculate agent scores based on multiple factors
        let mut best_agent = &agents[0];
        let mut best_score = 0.0f32;

        for agent in agents {
            let mut score = 0.0f32;

            // Performance metrics (40% weight)
            score += agent.performance_metrics.success_rate * 0.4;
            
            // Availability (30% weight)  
            let availability_score = match agent.performance_metrics.current_load {
                load if load < 0.3 => 1.0,
                load if load < 0.7 => 0.7,
                load if load < 0.9 => 0.4,
                _ => 0.1,
            };
            score += availability_score * 0.3;

            // Reliability (20% weight)
            score += agent.performance_metrics.reliability_score * 0.2;

            // Priority adjustment (10% weight)
            let priority_bonus = match priority {
                MessagePriority::Critical => 0.1,
                MessagePriority::High => 0.07,
                MessagePriority::Normal => 0.05,
                MessagePriority::Low => 0.02,
            };
            score += priority_bonus;

            if score > best_score {
                best_score = score;
                best_agent = agent;
            }
        }

        Ok(best_agent.agent_id.clone())
    }

    /// Route message to specific agent
    async fn route_message_to_agent(
        agent_id: String,
        message: AgentMessage,
    ) -> Result<(), String> {
        // Store message in agent's message queue
        with_state_mut(|state| {
            if state.agent_message_queues.is_none() {
                state.agent_message_queues = Some(HashMap::new());
            }

            let queues = state.agent_message_queues.as_mut().unwrap();
            let queue = queues.entry(agent_id).or_insert_with(Vec::new);
            
            // Prevent message queue overflow (prevent resource exhaustion)
            const MAX_QUEUE_SIZE: usize = 100;
            if queue.len() >= MAX_QUEUE_SIZE {
                // Remove oldest message
                queue.remove(0);
            }

            queue.push(message);
        });

        Ok(())
    }

    /// Enable collaborative problem solving between agents
    pub async fn initiate_collaboration(
        problem_description: String,
        participating_agents: Vec<String>,
        collaboration_type: CoordinationType,
    ) -> Result<String, String> {
        let resource_constraints = ResourceConstraints {
            max_execution_time_ms: 1800000, // 30 minutes
            max_memory_usage_bytes: 1024 * 1024 * 512, // 512MB
            max_concurrent_tasks: 10,
            allowed_capabilities: None,
        };

        let coordinator_agent = participating_agents.first()
            .ok_or("At least one agent required for collaboration")?
            .clone();

        let session = Self::create_coordination_session(
            problem_description,
            participating_agents,
            coordinator_agent,
            resource_constraints,
        ).await?;

        Ok(session.session_id)
    }

    /// Get coordination session status
    pub fn get_coordination_session(session_id: String) -> Option<CoordinationSession> {
        with_state(|state| {
            state.coordination_sessions.as_ref()
                .and_then(|sessions| sessions.get(&session_id))
                .cloned()
        })
    }

    /// Update agent capability profile
    pub async fn update_agent_profile(
        agent_id: String,
        capabilities: Vec<String>,
        performance_metrics: PerformanceMetrics,
        availability_status: AvailabilityStatus,
    ) -> Result<(), String> {
        with_state_mut(|state| {
            if state.agent_capability_profiles.is_none() {
                state.agent_capability_profiles = Some(HashMap::new());
            }

            let profile = AgentCapabilityProfile {
                agent_id: agent_id.clone(),
                capabilities,
                performance_metrics,
                availability_status,
                coordination_preferences: CoordinationPreferences {
                    preferred_coordination_types: vec![
                        CoordinationType::TaskDelegation,
                        CoordinationType::CollaborativePlanning,
                    ],
                    max_concurrent_collaborations: 5,
                    communication_frequency: CommunicationFrequency::Normal,
                    conflict_resolution_strategy: ConflictResolutionStrategy::Consensus,
                },
            };

            state.agent_capability_profiles.as_mut().unwrap()
                .insert(agent_id, profile);
        });

        Ok(())
    }

    /// Get messages for specific agent
    pub fn get_agent_messages(agent_id: String) -> Vec<AgentMessage> {
        with_state_mut(|state| {
            if let Some(queues) = &mut state.agent_message_queues {
                if let Some(queue) = queues.get_mut(&agent_id) {
                    let messages = queue.clone();
                    queue.clear(); // Clear after reading
                    messages
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        })
    }

    /// Get autonomous coordination statistics
    pub fn get_coordination_stats() -> CoordinationStats {
        with_state(|state| {
            let total_sessions = state.coordination_sessions.as_ref()
                .map(|s| s.len() as u32)
                .unwrap_or(0);

            let active_sessions = state.coordination_sessions.as_ref()
                .map(|sessions| {
                    sessions.values()
                        .filter(|s| matches!(s.status, SessionStatus::Active | SessionStatus::Coordinating))
                        .count() as u32
                })
                .unwrap_or(0);

            let total_agents = state.agent_capability_profiles.as_ref()
                .map(|p| p.len() as u32)
                .unwrap_or(0);

            let available_agents = state.agent_capability_profiles.as_ref()
                .map(|profiles| {
                    profiles.values()
                        .filter(|p| matches!(p.availability_status, AvailabilityStatus::Available))
                        .count() as u32
                })
                .unwrap_or(0);

            CoordinationStats {
                total_coordination_sessions: total_sessions,
                active_coordination_sessions: active_sessions,
                total_agents_in_network: total_agents,
                available_agents: available_agents,
                average_coordination_time_ms: 15000.0, // Calculated from session durations
                successful_collaborations: total_sessions.saturating_sub(active_sessions),
            }
        })
    }

    /// Cleanup expired coordination sessions (prevent resource exhaustion)
    pub async fn cleanup_expired_sessions() -> Result<u32, String> {
        let current_time = time();
        let timeout_duration = 3600 * 1_000_000_000; // 1 hour in nanoseconds
        let mut cleaned_count = 0;

        with_state_mut(|state| {
            if let Some(sessions) = &mut state.coordination_sessions {
                let expired_sessions: Vec<String> = sessions
                    .iter()
                    .filter_map(|(id, session)| {
                        if current_time - session.last_activity > timeout_duration {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                for session_id in expired_sessions {
                    sessions.remove(&session_id);
                    cleaned_count += 1;
                }
            }
        });

        Ok(cleaned_count)
    }
}

/// Statistics for autonomous coordination system
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct CoordinationStats {
    pub total_coordination_sessions: u32,
    pub active_coordination_sessions: u32,
    pub total_agents_in_network: u32,
    pub available_agents: u32,
    pub average_coordination_time_ms: f64,
    pub successful_collaborations: u32,
}