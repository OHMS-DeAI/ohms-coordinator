#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{with_state_mut, quota_manager::UserQuota};
    use ic_cdk::api::time;

    fn setup_test_state() {
        with_state_mut(|state| {
            // Clear existing state
            state.agents.clear();
            state.instruction_requests.clear();
            state.agent_creation_results.clear();
            state.user_quotas.clear();
            state.coordination_sessions = None;
            state.agent_capability_profiles = None;
        });
    }

    #[test]
    fn test_get_user_quota_status_new_user() {
        setup_test_state();
        
        // Simulate a new user calling the API
        let result = get_user_quota_status();
        assert!(result.is_ok());
        
        let quota = result.unwrap();
        assert_eq!(quota.tier, "Pro");
        assert_eq!(quota.remaining_agents, 25);
        assert_eq!(quota.monthly_limit, 25);
        assert!(quota.quota_available);
    }

    #[test]
    fn test_get_user_quota_status_existing_user() {
        setup_test_state();
        
        // Create existing user quota
        let user_principal = "test-user".to_string();
        let existing_quota = UserQuota {
            principal_id: user_principal.clone(),
            subscription_tier: "Basic".to_string(),
            limits: quota_manager::QuotaLimits {
                max_agents: 10,
                monthly_agent_creations: 15,
                token_limit: 2048,
                inference_rate: quota_manager::InferenceRate::Standard,
            },
            current_usage: quota_manager::QuotaUsage {
                agents_created_this_month: 5,
                tokens_used_this_month: 1000,
                inferences_this_month: 50,
                last_reset_date: time(),
            },
            last_updated: time(),
        };
        
        with_state_mut(|state| {
            state.user_quotas.insert(user_principal, existing_quota);
        });
        
        // Test quota status
        let result = get_user_quota_status();
        assert!(result.is_ok());
        
        let quota = result.unwrap();
        assert_eq!(quota.tier, "Basic");
        assert_eq!(quota.remaining_agents, 5); // 10 max - 5 created
        assert_eq!(quota.monthly_limit, 15);
        assert!(quota.quota_available);
    }

    #[test]
    fn test_list_user_agents() {
        setup_test_state();
        
        // Create test agents for different users
        let user1_agents = vec![
            AgentRegistration {
                agent_id: "agent1".to_string(),
                agent_principal: "user1".to_string(),
                canister_id: "canister1".to_string(),
                capabilities: vec!["coding".to_string()],
                model_id: "llama".to_string(),
                health_score: 1.0,
                registered_at: time(),
                last_seen: time(),
            },
            AgentRegistration {
                agent_id: "agent2".to_string(),
                agent_principal: "user1".to_string(),
                canister_id: "canister2".to_string(),
                capabilities: vec!["testing".to_string()],
                model_id: "llama".to_string(),
                health_score: 0.8,
                registered_at: time(),
                last_seen: time(),
            },
        ];
        
        let user2_agent = AgentRegistration {
            agent_id: "agent3".to_string(),
            agent_principal: "user2".to_string(),
            canister_id: "canister3".to_string(),
            capabilities: vec!["analysis".to_string()],
            model_id: "llama".to_string(),
            health_score: 0.9,
            registered_at: time(),
            last_seen: time(),
        };
        
        with_state_mut(|state| {
            for agent in user1_agents {
                state.agents.insert(agent.agent_id.clone(), agent);
            }
            state.agents.insert(user2_agent.agent_id.clone(), user2_agent);
        });
        
        // Test that user1 sees only their agents
        let result = list_user_agents();
        assert!(result.is_ok());
        
        let agents = result.unwrap();
        assert_eq!(agents.len(), 2);
        assert!(agents.iter().all(|agent| agent.agent_principal == "user1"));
    }

    #[test]
    fn test_get_agent_spawning_metrics() {
        setup_test_state();
        
        // Create test data
        let user_principal = "test-user".to_string();
        
        // Add instruction requests
        let request1 = InstructionRequest {
            request_id: "req1".to_string(),
            user_principal: user_principal.clone(),
            instructions: "Create a web app".to_string(),
            agent_count: Some(2),
            model_preferences: vec!["llama".to_string()],
            created_at: time(),
        };
        
        let request2 = InstructionRequest {
            request_id: "req2".to_string(),
            user_principal: "other-user".to_string(),
            instructions: "Create a mobile app".to_string(),
            agent_count: Some(1),
            model_preferences: vec!["mistral".to_string()],
            created_at: time(),
        };
        
        // Add agent creation results
        let result1 = AgentCreationResult {
            request_id: "req1".to_string(),
            created_agents: vec!["agent1".to_string(), "agent2".to_string()],
            creation_time_ms: 1500,
            status: AgentCreationStatus::Completed,
        };
        
        // Add agents
        let agent1 = AgentRegistration {
            agent_id: "agent1".to_string(),
            agent_principal: user_principal.clone(),
            canister_id: "canister1".to_string(),
            capabilities: vec!["coding".to_string()],
            model_id: "llama".to_string(),
            health_score: 1.0,
            registered_at: time(),
            last_seen: time(),
        };
        
        let agent2 = AgentRegistration {
            agent_id: "agent2".to_string(),
            agent_principal: user_principal.clone(),
            canister_id: "canister2".to_string(),
            capabilities: vec!["testing".to_string()],
            model_id: "llama".to_string(),
            health_score: 0.5, // Below threshold
            registered_at: time(),
            last_seen: time(),
        };
        
        with_state_mut(|state| {
            state.instruction_requests.insert("req1".to_string(), request1);
            state.instruction_requests.insert("req2".to_string(), request2);
            state.agent_creation_results.insert("req1".to_string(), result1);
            state.agents.insert("agent1".to_string(), agent1);
            state.agents.insert("agent2".to_string(), agent2);
        });
        
        // Test metrics
        let result = get_agent_spawning_metrics();
        assert!(result.is_ok());
        
        let metrics = result.unwrap();
        assert_eq!(metrics.total_instruction_requests, 2);
        assert_eq!(metrics.total_agent_creations, 1);
        assert_eq!(metrics.user_agents_created, 2);
        assert_eq!(metrics.user_active_agents, 1); // Only agent1 has health_score > 0.5
        assert_eq!(metrics.average_creation_time_ms, 1500);
        assert_eq!(metrics.success_rate, 0.95);
    }

    #[test]
    fn test_get_subscription_tier_info() {
        setup_test_state();
        
        // Create test user quota
        let user_principal = "test-user".to_string();
        let test_quota = UserQuota {
            principal_id: user_principal.clone(),
            subscription_tier: "Enterprise".to_string(),
            limits: quota_manager::QuotaLimits {
                max_agents: 100,
                monthly_agent_creations: 100,
                token_limit: 8192,
                inference_rate: quota_manager::InferenceRate::Premium,
            },
            current_usage: quota_manager::QuotaUsage {
                agents_created_this_month: 25,
                tokens_used_this_month: 4000,
                inferences_this_month: 200,
                last_reset_date: time(),
            },
            last_updated: time(),
        };
        
        with_state_mut(|state| {
            state.user_quotas.insert(user_principal, test_quota);
        });
        
        // Test tier info
        let result = get_subscription_tier_info();
        assert!(result.is_ok());
        
        let tier_info = result.unwrap();
        assert_eq!(tier_info.current_tier, "Enterprise");
        assert_eq!(tier_info.max_agents, 100);
        assert_eq!(tier_info.monthly_creations, 100);
        assert_eq!(tier_info.token_limit, 8192);
        assert_eq!(tier_info.inference_rate, "Premium");
        assert_eq!(tier_info.agents_created_this_month, 25);
        assert_eq!(tier_info.tokens_used_this_month, 4000);
    }

    #[test]
    fn test_upgrade_subscription_tier() {
        setup_test_state();
        
        // Create initial user quota
        let user_principal = "test-user".to_string();
        let initial_quota = UserQuota {
            principal_id: user_principal.clone(),
            subscription_tier: "Basic".to_string(),
            limits: quota_manager::QuotaLimits {
                max_agents: 10,
                monthly_agent_creations: 15,
                token_limit: 2048,
                inference_rate: quota_manager::InferenceRate::Standard,
            },
            current_usage: quota_manager::QuotaUsage {
                agents_created_this_month: 5,
                tokens_used_this_month: 1000,
                inferences_this_month: 50,
                last_reset_date: time(),
            },
            last_updated: time(),
        };
        
        with_state_mut(|state| {
            state.user_quotas.insert(user_principal.clone(), initial_quota);
        });
        
        // Upgrade to Pro tier
        let result = upgrade_subscription_tier("Pro".to_string());
        assert!(result.is_ok());
        
        // Verify upgrade
        let tier_info = get_subscription_tier_info().unwrap();
        assert_eq!(tier_info.current_tier, "Pro");
        assert_eq!(tier_info.max_agents, 25);
        assert_eq!(tier_info.monthly_creations, 25);
        assert_eq!(tier_info.token_limit, 4096);
        assert_eq!(tier_info.inference_rate, "Priority");
        
        // Test invalid tier
        let invalid_result = upgrade_subscription_tier("Invalid".to_string());
        assert!(invalid_result.is_err());
    }
}
