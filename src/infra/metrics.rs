use ic_cdk::api::time;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static METRICS: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
}

pub struct Metrics;

impl Metrics {
    pub fn increment_counter(name: &str) {
        METRICS.with(|m| {
            let mut metrics = m.borrow_mut();
            *metrics.entry(name.to_string()).or_insert(0) += 1;
        });
    }
    
    pub fn get_counter(name: &str) -> u64 {
        METRICS.with(|m| {
            m.borrow().get(name).copied().unwrap_or(0)
        })
    }
}