use crate::domain::*;
use crate::services::{with_state, with_state_mut};
use ic_cdk::api::time;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};

pub struct DedupService;

impl DedupService {
    const TTL_DURATION: u64 = 24 * 60 * 60 * 1_000_000_000; // 24 hours in nanoseconds
    
    pub fn is_duplicate(msg_id: &str) -> bool {
        let now = time();
        
        with_state_mut(|state| {
            // Clean expired entries first
            state.dedup_cache.retain(|_, entry| entry.ttl_expires_at > now);
            
            // Check if message ID exists and is not expired
            state.dedup_cache.contains_key(msg_id)
        })
    }
    
    pub fn record_request(msg_id: &str, response: &RouteResponse) -> Result<(), String> {
        let now = time();
        let result_hash = Self::hash_response(response);
        
        let entry = DedupEntry {
            msg_id: msg_id.to_string(),
            processed_at: now,
            result_hash,
            ttl_expires_at: now + Self::TTL_DURATION,
        };
        
        with_state_mut(|state| {
            state.dedup_cache.insert(msg_id.to_string(), entry);
        });
        
        Ok(())
    }
    
    pub fn get_cached_result(msg_id: &str) -> Option<String> {
        let now = time();
        
        with_state(|state| {
            state.dedup_cache
                .get(msg_id)
                .filter(|entry| entry.ttl_expires_at > now)
                .map(|entry| entry.result_hash.clone())
        })
    }
    
    pub fn cleanup_expired() -> u32 {
        let now = time();
        
        with_state_mut(|state| {
            let initial_count = state.dedup_cache.len();
            state.dedup_cache.retain(|_, entry| entry.ttl_expires_at > now);
            let final_count = state.dedup_cache.len();
            
            (initial_count - final_count) as u32
        })
    }
    
    pub fn get_cache_stats() -> (u32, u32) {
        let now = time();
        
        with_state(|state| {
            let total = state.dedup_cache.len() as u32;
            let expired = state.dedup_cache
                .values()
                .filter(|entry| entry.ttl_expires_at <= now)
                .count() as u32;
            
            (total, expired)
        })
    }
    
    fn hash_response(response: &RouteResponse) -> String {
        let mut hasher = Sha256::new();
        hasher.update(response.request_id.as_bytes());
        hasher.update(response.selected_agents.join(",").as_bytes());
        hasher.update(response.routing_time_ms.to_be_bytes());
        let hash = hasher.finalize();
        general_purpose::STANDARD.encode(&hash[..16])
    }
}