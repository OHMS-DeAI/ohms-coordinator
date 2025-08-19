use ic_cdk::api::time;
use serde::{Deserialize, Serialize};
use candid::CandidType;
use std::collections::HashMap;
use crate::services::{with_state, with_state_mut};

/// Quota manager service for enforcing subscription limits
pub struct QuotaManager;

/// User quota tracking and enforcement
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct UserQuota {
    pub principal_id: String,
    pub subscription_tier: String,
    pub current_usage: QuotaUsage,
    pub limits: QuotaLimits,
    pub last_updated: u64,
}

/// Current usage tracking
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct QuotaUsage {
    pub agents_created_this_month: u32,
    pub tokens_used_this_month: u64,
    pub inferences_this_month: u32,
    pub last_reset_date: u64,
}

/// Quota limits based on subscription tier
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct QuotaLimits {
    pub max_agents: u32,
    pub monthly_agent_creations: u32,
    pub token_limit: u64,
    pub inference_rate: InferenceRate,
}

/// Inference rate priority levels
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum InferenceRate {
    Standard,
    Priority,
    Premium,
}

/// Quota validation result
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct QuotaValidation {
    pub allowed: bool,
    pub reason: Option<String>,
    pub remaining_quota: Option<QuotaRemaining>,
}

/// Remaining quota information
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct QuotaRemaining {
    pub agents_remaining: u32,
    pub tokens_remaining: u64,
    pub inferences_remaining: u32,
}

/// Quota enforcement action
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub enum QuotaAction {
    AgentCreation,
    TokenUsage,
    Inference,
}

impl QuotaManager {
    /// Initialize user quota tracking
    pub fn initialize_user_quota(
        principal_id: String,
        subscription_tier: String,
        limits: QuotaLimits,
    ) -> Result<(), String> {
        let now = time();
        
        let user_quota = UserQuota {
            principal_id: principal_id.clone(),
            subscription_tier,
            current_usage: QuotaUsage {
                agents_created_this_month: 0,
                tokens_used_this_month: 0,
                inferences_this_month: 0,
                last_reset_date: now,
            },
            limits,
            last_updated: now,
        };

        with_state_mut(|state| {
            state.user_quotas.insert(principal_id, user_quota);
        });

        Ok(())
    }

    /// Validate quota for a specific action
    pub fn validate_quota(
        principal_id: &str,
        action: QuotaAction,
        amount: Option<u64>,
    ) -> Result<QuotaValidation, String> {
        let mut user_quota = Self::get_user_quota(principal_id)
            .ok_or("No quota found for user")?;

        // Reset monthly usage if needed
        Self::reset_monthly_usage_if_needed(&mut user_quota);

        let validation = match action {
            QuotaAction::AgentCreation => {
                Self::validate_agent_creation_quota(&user_quota)
            },
            QuotaAction::TokenUsage => {
                let tokens = amount.ok_or("Token amount required")?;
                Self::validate_token_usage_quota(&user_quota, tokens)
            },
            QuotaAction::Inference => {
                Self::validate_inference_quota(&user_quota)
            },
        };

        // Update usage if validation passed
        if validation.allowed {
            Self::update_usage(&mut user_quota, &action, amount);
            Self::store_user_quota(user_quota);
        }

        Ok(validation)
    }

    /// Validate agent creation quota
    fn validate_agent_creation_quota(user_quota: &UserQuota) -> QuotaValidation {
        if user_quota.current_usage.agents_created_this_month >= user_quota.limits.monthly_agent_creations {
            return QuotaValidation {
                allowed: false,
                reason: Some("Monthly agent creation limit reached".to_string()),
                remaining_quota: Some(QuotaRemaining {
                    agents_remaining: 0,
                    tokens_remaining: user_quota.limits.token_limit.saturating_sub(user_quota.current_usage.tokens_used_this_month),
                    inferences_remaining: 0,
                }),
            };
        }

        QuotaValidation {
            allowed: true,
            reason: None,
            remaining_quota: Some(QuotaRemaining {
                agents_remaining: user_quota.limits.monthly_agent_creations.saturating_sub(user_quota.current_usage.agents_created_this_month),
                tokens_remaining: user_quota.limits.token_limit.saturating_sub(user_quota.current_usage.tokens_used_this_month),
                inferences_remaining: 0,
            }),
        }
    }

    /// Validate token usage quota
    fn validate_token_usage_quota(user_quota: &UserQuota, tokens_requested: u64) -> QuotaValidation {
        let remaining_tokens = user_quota.limits.token_limit.saturating_sub(user_quota.current_usage.tokens_used_this_month);
        
        if tokens_requested > remaining_tokens {
            return QuotaValidation {
                allowed: false,
                reason: Some("Insufficient token quota".to_string()),
                remaining_quota: Some(QuotaRemaining {
                    agents_remaining: user_quota.limits.monthly_agent_creations.saturating_sub(user_quota.current_usage.agents_created_this_month),
                    tokens_remaining: remaining_tokens,
                    inferences_remaining: 0,
                }),
            };
        }

        QuotaValidation {
            allowed: true,
            reason: None,
            remaining_quota: Some(QuotaRemaining {
                agents_remaining: user_quota.limits.monthly_agent_creations.saturating_sub(user_quota.current_usage.agents_created_this_month),
                tokens_remaining: remaining_tokens,
                inferences_remaining: 0,
            }),
        }
    }

    /// Validate inference quota
    fn validate_inference_quota(user_quota: &UserQuota) -> QuotaValidation {
        // For now, inference is unlimited but rate-limited
        QuotaValidation {
            allowed: true,
            reason: None,
            remaining_quota: Some(QuotaRemaining {
                agents_remaining: user_quota.limits.monthly_agent_creations.saturating_sub(user_quota.current_usage.agents_created_this_month),
                tokens_remaining: user_quota.limits.token_limit.saturating_sub(user_quota.current_usage.tokens_used_this_month),
                inferences_remaining: 0,
            }),
        }
    }

    /// Update usage after successful validation
    fn update_usage(user_quota: &mut UserQuota, action: &QuotaAction, amount: Option<u64>) {
        match action {
            QuotaAction::AgentCreation => {
                user_quota.current_usage.agents_created_this_month += 1;
            },
            QuotaAction::TokenUsage => {
                if let Some(tokens) = amount {
                    user_quota.current_usage.tokens_used_this_month += tokens;
                }
            },
            QuotaAction::Inference => {
                user_quota.current_usage.inferences_this_month += 1;
            },
        }
        user_quota.last_updated = time();
    }

    /// Get user quota
    pub fn get_user_quota(principal_id: &str) -> Option<UserQuota> {
        with_state(|state| {
            state.user_quotas.get(principal_id).cloned()
        })
    }

    /// Store user quota
    fn store_user_quota(user_quota: UserQuota) {
        with_state_mut(|state| {
            state.user_quotas.insert(user_quota.principal_id.clone(), user_quota);
        });
    }

    /// Reset monthly usage if a new month has started
    fn reset_monthly_usage_if_needed(user_quota: &mut UserQuota) {
        let now = time();
        let last_reset = user_quota.current_usage.last_reset_date;
        
        // Check if we're in a new month (simple check: 30 days passed)
        if now - last_reset > 30 * 24 * 60 * 60 * 1_000_000_000 {
            user_quota.current_usage = QuotaUsage {
                agents_created_this_month: 0,
                tokens_used_this_month: 0,
                inferences_this_month: 0,
                last_reset_date: now,
            };
        }
    }

    /// Get user usage metrics
    pub fn get_user_usage(principal_id: &str) -> Option<QuotaUsage> {
        Self::get_user_quota(principal_id)
            .map(|quota| quota.current_usage)
    }

    /// Update user quota limits (for subscription changes)
    pub fn update_user_quota_limits(
        principal_id: String,
        new_limits: QuotaLimits,
    ) -> Result<(), String> {
        with_state_mut(|state| {
            if let Some(quota) = state.user_quotas.get_mut(&principal_id) {
                quota.limits = new_limits;
                quota.last_updated = time();
            }
        });
        Ok(())
    }

    /// List all user quotas (admin only)
    pub fn list_all_user_quotas() -> Vec<UserQuota> {
        with_state(|state| {
            state.user_quotas.values().cloned().collect()
        })
    }

    /// Get quota statistics (admin only)
    pub fn get_quota_stats() -> QuotaStats {
        let quotas = Self::list_all_user_quotas();
        
        let mut stats = QuotaStats {
            total_users: quotas.len() as u32,
            tier_distribution: HashMap::new(),
            total_agents_created: 0,
            total_tokens_used: 0,
            total_inferences: 0,
        };

        for quota in quotas {
            // Count by tier
            let tier_name = quota.subscription_tier.clone();
            *stats.tier_distribution.entry(tier_name).or_insert(0) += 1;

            // Aggregate usage
            stats.total_agents_created += quota.current_usage.agents_created_this_month;
            stats.total_tokens_used += quota.current_usage.tokens_used_this_month;
            stats.total_inferences += quota.current_usage.inferences_this_month;
        }

        stats
    }
}

/// Quota statistics for admin dashboard
#[derive(Debug, Clone, Serialize, Deserialize, CandidType)]
pub struct QuotaStats {
    pub total_users: u32,
    pub tier_distribution: HashMap<String, u32>,
    pub total_agents_created: u32,
    pub total_tokens_used: u64,
    pub total_inferences: u32,
}
