use crate::domain::*;
use crate::services::{with_state, with_state_mut};
use ic_cdk::api::time;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

pub struct BountyService;

impl BountyService {
    pub async fn open_bounty(spec: BountySpec, escrow_id: String) -> Result<String, String> {
        let now = time();
        let bounty_id = Self::generate_bounty_id(&spec.title, &escrow_id);
        
        let bounty = Bounty {
            bounty_id: bounty_id.clone(),
            spec,
            creator: "caller_principal".to_string(), // TODO: Get actual caller
            escrow_id,
            status: BountyStatus::Open,
            created_at: now,
            submissions: Vec::new(),
        };
        
        with_state_mut(|state| {
            state.bounties.insert(bounty_id.clone(), bounty);
            state.metrics.total_bounties += 1;
            state.metrics.last_activity = now;
        });
        
        Ok(bounty_id)
    }
    
    pub async fn submit_result(bounty_id: String, agent_id: String, payload: Vec<u8>) -> Result<String, String> {
        let now = time();
        let submission_id = Self::generate_submission_id(&bounty_id, &agent_id);
        
        with_state_mut(|state| {
            if let Some(bounty) = state.bounties.get_mut(&bounty_id) {
                if !matches!(bounty.status, BountyStatus::Open) {
                    return Err("Bounty is not accepting submissions".to_string());
                }
                
                if bounty.spec.deadline_timestamp < now {
                    bounty.status = BountyStatus::Expired;
                    return Err("Bounty deadline has passed".to_string());
                }
                
                let submission = BountySubmission {
                    submission_id: submission_id.clone(),
                    bounty_id,
                    agent_id,
                    payload,
                    submitted_at: now,
                    evaluation_score: None,
                };
                
                bounty.submissions.push(submission);
                bounty.status = BountyStatus::InProgress;
                
                Ok(submission_id)
            } else {
                Err("Bounty not found".to_string())
            }
        })
    }
    
    pub async fn resolve_bounty(bounty_id: String, winner_id: Option<String>) -> Result<BountyResolution, String> {
        let now = time();
        
        with_state_mut(|state| {
            if let Some(bounty) = state.bounties.get_mut(&bounty_id) {
                let resolution_type = match winner_id.as_ref() {
                    Some(_) => ResolutionType::WinnerSelected,
                    None => ResolutionType::NoWinner,
                };
                
                bounty.status = BountyStatus::Resolved;
                
                let resolution = BountyResolution {
                    bounty_id,
                    winner_id,
                    resolution_type,
                    resolved_at: now,
                    settlement_details: "Automated resolution".to_string(),
                };
                
                Ok(resolution)
            } else {
                Err("Bounty not found".to_string())
            }
        })
    }
    
    pub fn get_bounty(bounty_id: &str) -> Result<Bounty, String> {
        with_state(|state| {
            state.bounties
                .get(bounty_id)
                .cloned()
                .ok_or_else(|| format!("Bounty not found: {}", bounty_id))
        })
    }
    
    pub fn list_bounties() -> Vec<Bounty> {
        with_state(|state| state.bounties.values().cloned().collect())
    }
    
    fn generate_bounty_id(title: &str, escrow_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(escrow_id.as_bytes());
        hasher.update(time().to_be_bytes());
        let hash = hasher.finalize();
        format!("bounty_{}", general_purpose::STANDARD.encode(&hash[..8]))
    }
    
    fn generate_submission_id(bounty_id: &str, agent_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bounty_id.as_bytes());
        hasher.update(agent_id.as_bytes());
        hasher.update(time().to_be_bytes());
        let hash = hasher.finalize();
        format!("submission_{}", general_purpose::STANDARD.encode(&hash[..8]))
    }
}