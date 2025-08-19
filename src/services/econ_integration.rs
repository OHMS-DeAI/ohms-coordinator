use crate::domain::*;
use crate::services::{with_state_mut};
use ic_cdk::api::{call, time};
use candid::Principal;
use serde::{Deserialize, Serialize};
use candid::CandidType;

/// Economics canister integration service for OHMS 2.0 subscription management
pub struct EconIntegrationService;

/// Cross-canister call types for economics integration
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct UserSubscription {
    pub principal_id: String,
    pub tier: TierConfig,
    pub started_at: u64,
    pub expires_at: u64,
    pub auto_renew: bool,
    pub current_usage: UsageMetrics,
    pub payment_status: PaymentStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct TierConfig {
    pub name: String,
    pub monthly_fee_usd: u32,
    pub max_agents: u32,
    pub monthly_agent_creations: u32,
    pub token_limit: u64,
    pub inference_rate: InferenceRate,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum InferenceRate {
    Standard,
    Priority,
    Premium,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum PaymentStatus {
    Active,
    Pending,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct UsageMetrics {
    pub agents_created_this_month: u32,
    pub tokens_used_this_month: u64,
    pub inferences_this_month: u32,
    pub last_reset_date: u64,
}

impl EconIntegrationService {
    /// Get the economics canister ID
    fn get_econ_canister_id() -> Principal {
        // Use the actual economics canister ID from deployment
        Principal::from_text("tetse-piaaa-aaaao-qkeyq-cai").unwrap_or_else(|_| Principal::anonymous())
    }

    /// Validate user subscription and quota for agent creation
    pub async fn validate_agent_creation_quota(user_principal: &str) -> Result<QuotaValidation, String> {
        let econ_canister_id = Self::get_econ_canister_id();
        
        // Make cross-canister call to validate quota
        match call::call::<_, (Result<QuotaValidation, String>,)>(
            econ_canister_id,
            "validate_agent_creation_quota",
            (),
        ).await {
            Ok((Ok(validation),)) => Ok(validation),
            Ok((Err(e),)) => Err(format!("Economics canister error: {}", e)),
            Err(e) => Err(format!("Cross-canister call failed: {:?}", e)),
        }
    }

    /// Validate token usage quota for inference
    pub async fn validate_token_usage_quota(user_principal: &str, tokens: u64) -> Result<QuotaValidation, String> {
        let econ_canister_id = Self::get_econ_canister_id();
        
        // Make cross-canister call to validate token usage
        match call::call::<_, (Result<QuotaValidation, String>,)>(
            econ_canister_id,
            "validate_token_usage_quota",
            (tokens,),
        ).await {
            Ok((Ok(validation),)) => Ok(validation),
            Ok((Err(e),)) => Err(format!("Economics canister error: {}", e)),
            Err(e) => Err(format!("Cross-canister call failed: {:?}", e)),
        }
    }

    /// Get user subscription details
    pub async fn get_user_subscription(user_principal: &str) -> Result<Option<UserSubscription>, String> {
        let econ_canister_id = Self::get_econ_canister_id();
        
        // Make cross-canister call to get subscription
        match call::call::<_, (Option<UserSubscription>,)>(
            econ_canister_id,
            "get_user_subscription",
            (Some(user_principal.to_string()),),
        ).await {
            Ok((subscription,)) => Ok(subscription),
            Err(e) => Err(format!("Cross-canister call failed: {:?}", e)),
        }
    }

    /// Create or get free subscription for new users
    pub async fn get_or_create_free_subscription(user_principal: &str) -> Result<UserSubscription, String> {
        let econ_canister_id = Self::get_econ_canister_id();
        
        // Make cross-canister call to create/get free subscription
        match call::call::<_, (Result<UserSubscription, String>,)>(
            econ_canister_id,
            "get_or_create_free_subscription",
            (),
        ).await {
            Ok((Ok(subscription),)) => Ok(subscription),
            Ok((Err(e),)) => Err(format!("Economics canister error: {}", e)),
            Err(e) => Err(format!("Cross-canister call failed: {:?}", e)),
        }
    }

    /// Update local quota cache with economics data
    pub async fn sync_user_quota_from_economics(user_principal: &str) -> Result<(), String> {
        let subscription = Self::get_user_subscription(user_principal).await?;
        
        match subscription {
            Some(sub) => {
                // Convert economics subscription to local quota format
                let local_quota = crate::services::quota_manager::UserQuota {
                    principal_id: user_principal.to_string(),
                    subscription_tier: sub.tier.name,
                    limits: crate::services::quota_manager::QuotaLimits {
                        max_agents: sub.tier.max_agents,
                        monthly_agent_creations: sub.tier.monthly_agent_creations,
                        token_limit: sub.tier.token_limit,
                        inference_rate: match sub.tier.inference_rate {
                            InferenceRate::Standard => crate::services::quota_manager::InferenceRate::Standard,
                            InferenceRate::Priority => crate::services::quota_manager::InferenceRate::Priority,
                            InferenceRate::Premium => crate::services::quota_manager::InferenceRate::Premium,
                        },
                    },
                    current_usage: crate::services::quota_manager::QuotaUsage {
                        agents_created_this_month: sub.current_usage.agents_created_this_month,
                        tokens_used_this_month: sub.current_usage.tokens_used_this_month,
                        inferences_this_month: sub.current_usage.inferences_this_month,
                        last_reset_date: sub.current_usage.last_reset_date,
                    },
                    last_updated: time(),
                };
                
                // Update local state
                with_state_mut(|state| {
                    state.user_quotas.insert(user_principal.to_string(), local_quota);
                });
                
                Ok(())
            },
            None => {
                // Create free subscription if none exists
                let _free_sub = Self::get_or_create_free_subscription(user_principal).await?;
                
                // Get the subscription again after creation
                let subscription = Self::get_user_subscription(user_principal).await?;
                
                if let Some(sub) = subscription {
                    // Convert economics subscription to local quota format
                    let local_quota = crate::services::quota_manager::UserQuota {
                        principal_id: user_principal.to_string(),
                        subscription_tier: sub.tier.name,
                        limits: crate::services::quota_manager::QuotaLimits {
                            max_agents: sub.tier.max_agents,
                            monthly_agent_creations: sub.tier.monthly_agent_creations,
                            token_limit: sub.tier.token_limit,
                            inference_rate: match sub.tier.inference_rate {
                                InferenceRate::Standard => crate::services::quota_manager::InferenceRate::Standard,
                                InferenceRate::Priority => crate::services::quota_manager::InferenceRate::Priority,
                                InferenceRate::Premium => crate::services::quota_manager::InferenceRate::Premium,
                            },
                        },
                        current_usage: crate::services::quota_manager::QuotaUsage {
                            agents_created_this_month: sub.current_usage.agents_created_this_month,
                            tokens_used_this_month: sub.current_usage.tokens_used_this_month,
                            inferences_this_month: sub.current_usage.inferences_this_month,
                            last_reset_date: sub.current_usage.last_reset_date,
                        },
                        last_updated: time(),
                    };
                    
                    // Update local state
                    with_state_mut(|state| {
                        state.user_quotas.insert(user_principal.to_string(), local_quota);
                    });
                    
                    Ok(())
                } else {
                    Err("Failed to create user subscription".to_string())
                }
            }
        }
    }

    /// Check if user has active subscription
    pub async fn has_active_subscription(user_principal: &str) -> Result<bool, String> {
        let subscription = Self::get_user_subscription(user_principal).await?;
        
        match subscription {
            Some(sub) => {
                let now = time();
                let is_active = sub.expires_at > now && 
                               matches!(sub.payment_status, PaymentStatus::Active);
                Ok(is_active)
            },
            None => Ok(false),
        }
    }

    /// Get subscription tier limits
    pub async fn get_subscription_limits(user_principal: &str) -> Result<TierConfig, String> {
        let subscription = Self::get_user_subscription(user_principal).await?;
        
        match subscription {
            Some(sub) => Ok(sub.tier),
            None => {
                // Return free tier limits if no subscription
                Ok(TierConfig {
                    name: "Free".to_string(),
                    monthly_fee_usd: 0,
                    max_agents: 3,
                    monthly_agent_creations: 5,
                    token_limit: 1024,
                    inference_rate: InferenceRate::Standard,
                    features: vec!["Basic agent creation".to_string()],
                })
            }
        }
    }

    /// Track agent creation in economics canister
    pub async fn track_agent_creation(user_principal: &str, agent_count: u32) -> Result<(), String> {
        // This would typically update usage metrics in the economics canister
        // For now, we'll just sync the quota to ensure consistency
        Self::sync_user_quota_from_economics(user_principal).await
    }

    /// Track token usage in economics canister
    pub async fn track_token_usage(user_principal: &str, tokens: u64) -> Result<(), String> {
        // This would typically update usage metrics in the economics canister
        // For now, we'll just sync the quota to ensure consistency
        Self::sync_user_quota_from_economics(user_principal).await
    }

    /// Get economics canister health
    pub async fn get_economics_health() -> Result<EconHealth, String> {
        let econ_canister_id = Self::get_econ_canister_id();
        
        match call::call::<_, (EconHealth,)>(
            econ_canister_id,
            "health",
            (),
        ).await {
            Ok((health,)) => Ok(health),
            Err(e) => Err(format!("Cross-canister call failed: {:?}", e)),
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_econ_canister_id() {
        let canister_id = EconIntegrationService::get_econ_canister_id();
        // Use the real economics canister ID
        assert_eq!(canister_id.to_text(), "tetse-piaaa-aaaao-qkeyq-cai");
    }

    #[test]
    fn test_subscription_limits_free_tier() {
        let free_tier = TierConfig {
            name: "Free".to_string(),
            monthly_fee_usd: 0,
            max_agents: 3,
            monthly_agent_creations: 5,
            token_limit: 1024,
            inference_rate: InferenceRate::Standard,
            features: vec!["Basic agent creation".to_string()],
        };
        
        assert_eq!(free_tier.name, "Free");
        assert_eq!(free_tier.max_agents, 3);
        assert_eq!(free_tier.monthly_agent_creations, 5);
        assert_eq!(free_tier.token_limit, 1024);
    }

    #[test]
    fn test_quota_validation_structure() {
        let validation = QuotaValidation {
            allowed: true,
            reason: None,
            remaining_quota: Some(QuotaRemaining {
                agents_remaining: 5,
                tokens_remaining: 1000,
                inferences_remaining: 50,
            }),
        };
        
        assert!(validation.allowed);
        assert!(validation.reason.is_none());
        assert!(validation.remaining_quota.is_some());
        
        if let Some(quota) = validation.remaining_quota {
            assert_eq!(quota.agents_remaining, 5);
            assert_eq!(quota.tokens_remaining, 1000);
            assert_eq!(quota.inferences_remaining, 50);
        }
    }
}
