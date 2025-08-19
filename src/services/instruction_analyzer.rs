use crate::domain::*;
use crate::services::{with_state, with_state_mut};
use ic_cdk::api::time;

/// Instruction analysis service for OHMS 2.0 agent spawning
pub struct InstructionAnalyzerService;

/// Parsed instruction requirements
#[derive(Debug, Clone)]
pub struct ParsedRequirements {
    pub agent_count: u32,
    pub required_capabilities: Vec<String>,
    pub model_requirements: Vec<String>,
    pub specializations: Vec<String>,
    pub coordination_needs: Vec<String>,
    pub complexity_level: ComplexityLevel,
}

/// Complexity levels for instruction analysis
#[derive(Debug, Clone, PartialEq)]
pub enum ComplexityLevel {
    Simple,     // Single agent, basic task
    Moderate,   // 2-3 agents, some coordination
    Complex,    // 4+ agents, significant coordination
    Enterprise, // Multi-team coordination
}

/// Capability patterns for instruction parsing
#[derive(Debug, Clone)]
pub struct CapabilityPattern {
    pub keywords: Vec<String>,
    pub capabilities: Vec<String>,
    pub model_suggestions: Vec<String>,
    pub specialization: String,
}

impl InstructionAnalyzerService {
    /// Analyze natural language instructions and determine agent requirements
    pub fn analyze_instructions(instructions: &str, user_principal: &str) -> Result<InstructionAnalysisResult, String> {
        let request_id = format!("analysis_{}", time());
        
        // Parse the instructions
        let parsed = Self::parse_instructions(instructions)?;
        
        // Check user quotas
        let quota_check = Self::check_user_quotas(user_principal, parsed.agent_count)?;
        
        // Generate agent specifications
        let suggested_agents = Self::generate_agent_specs(&parsed)?;
        
        // Create coordination plan
        let coordination_plan = Self::create_coordination_plan(&parsed, &suggested_agents)?;
        
        let result = InstructionAnalysisResult {
            request_id,
            parsed_requirements: parsed.required_capabilities,
            suggested_agents,
            coordination_plan,
            quota_check,
        };
        
        Ok(result)
    }
    
    /// Parse natural language instructions into structured requirements
    fn parse_instructions(instructions: &str) -> Result<ParsedRequirements, String> {
        let instructions_lower = instructions.to_lowercase();
        
        // Initialize capability patterns
        let patterns = Self::get_capability_patterns();
        
        let mut required_capabilities = Vec::new();
        let mut model_requirements = Vec::new();
        let mut specializations = Vec::new();
        let mut coordination_needs = Vec::new();
        
        // Analyze instructions against patterns
        for pattern in &patterns {
            if Self::matches_pattern(&instructions_lower, &pattern.keywords) {
                required_capabilities.extend(pattern.capabilities.clone());
                model_requirements.extend(pattern.model_suggestions.clone());
                specializations.push(pattern.specialization.clone());
            }
        }
        
        // Determine agent count based on complexity
        let agent_count = Self::determine_agent_count(&instructions_lower, &required_capabilities);
        
        // Determine coordination needs
        coordination_needs = Self::determine_coordination_needs(&instructions_lower, agent_count);
        
        // Determine complexity level
        let complexity_level = Self::determine_complexity_level(agent_count, &coordination_needs);
        
        Ok(ParsedRequirements {
            agent_count,
            required_capabilities,
            model_requirements,
            specializations,
            coordination_needs,
            complexity_level,
        })
    }
    
    /// Get predefined capability patterns for instruction parsing
    fn get_capability_patterns() -> Vec<CapabilityPattern> {
        vec![
            // Development patterns
            CapabilityPattern {
                keywords: vec!["code", "programming", "develop", "software", "application"].into_iter().map(|s| s.to_string()).collect(),
                capabilities: vec!["coding", "software_development", "programming"].into_iter().map(|s| s.to_string()).collect(),
                model_suggestions: vec!["code-llama", "starcoder", "wizardcoder"].into_iter().map(|s| s.to_string()).collect(),
                specialization: "Software Developer".to_string(),
            },
            CapabilityPattern {
                keywords: vec!["test", "testing", "qa", "quality", "verify"].into_iter().map(|s| s.to_string()).collect(),
                capabilities: vec!["testing", "quality_assurance", "verification"].into_iter().map(|s| s.to_string()).collect(),
                model_suggestions: vec!["code-llama", "starcoder"].into_iter().map(|s| s.to_string()).collect(),
                specialization: "Test Engineer".to_string(),
            },
            CapabilityPattern {
                keywords: vec!["review", "code review", "peer review"].into_iter().map(|s| s.to_string()).collect(),
                capabilities: vec!["code_review", "quality_assurance", "best_practices"].into_iter().map(|s| s.to_string()).collect(),
                model_suggestions: vec!["code-llama", "starcoder"].into_iter().map(|s| s.to_string()).collect(),
                specialization: "Code Reviewer".to_string(),
            },
            
            // Content creation patterns
            CapabilityPattern {
                keywords: vec!["write", "content", "article", "blog", "documentation"].into_iter().map(|s| s.to_string()).collect(),
                capabilities: vec!["content_creation", "writing", "documentation"].into_iter().map(|s| s.to_string()).collect(),
                model_suggestions: vec!["llama", "mistral", "gemma"].into_iter().map(|s| s.to_string()).collect(),
                specialization: "Content Creator".to_string(),
            },
            CapabilityPattern {
                keywords: vec!["marketing", "social media", "campaign", "promote"].into_iter().map(|s| s.to_string()).collect(),
                capabilities: vec!["marketing", "social_media", "campaign_management"].into_iter().map(|s| s.to_string()).collect(),
                model_suggestions: vec!["llama", "mistral"].into_iter().map(|s| s.to_string()).collect(),
                specialization: "Marketing Specialist".to_string(),
            },
            
            // Data analysis patterns
            CapabilityPattern {
                keywords: vec!["analyze", "data", "analytics", "insights", "report"].into_iter().map(|s| s.to_string()).collect(),
                capabilities: vec!["data_analysis", "analytics", "reporting"].into_iter().map(|s| s.to_string()).collect(),
                model_suggestions: vec!["llama", "mistral", "gemma"].into_iter().map(|s| s.to_string()).collect(),
                specialization: "Data Analyst".to_string(),
            },
            
            // Research patterns
            CapabilityPattern {
                keywords: vec!["research", "investigate", "study", "explore"].into_iter().map(|s| s.to_string()).collect(),
                capabilities: vec!["research", "investigation", "analysis"].into_iter().map(|s| s.to_string()).collect(),
                model_suggestions: vec!["llama", "mistral", "gemma"].into_iter().map(|s| s.to_string()).collect(),
                specialization: "Research Analyst".to_string(),
            },
        ]
    }
    
    /// Check if instructions match a capability pattern
    fn matches_pattern(instructions: &str, keywords: &[String]) -> bool {
        keywords.iter().any(|keyword| instructions.contains(keyword))
    }
    
    /// Determine number of agents needed based on instruction complexity
    fn determine_agent_count(instructions: &str, capabilities: &[String]) -> u32 {
        let capability_count = capabilities.len() as u32;
        
        // Base count on capabilities
        let mut agent_count = capability_count.max(1);
        
        // Adjust based on instruction complexity indicators
        if instructions.contains("team") || instructions.contains("multiple") {
            agent_count = agent_count.max(3);
        }
        
        if instructions.contains("complex") || instructions.contains("comprehensive") {
            agent_count = agent_count.max(4);
        }
        
        // Cap at reasonable limit
        agent_count.min(10)
    }
    
    /// Determine coordination needs based on agent count and instructions
    fn determine_coordination_needs(instructions: &str, agent_count: u32) -> Vec<String> {
        let mut needs = Vec::new();
        
        if agent_count > 1 {
            needs.push("inter_agent_communication".to_string());
        }
        
        if instructions.contains("collaborate") || instructions.contains("coordinate") {
            needs.push("task_coordination".to_string());
        }
        
        if instructions.contains("review") || instructions.contains("approve") {
            needs.push("workflow_approval".to_string());
        }
        
        if agent_count > 3 {
            needs.push("load_balancing".to_string());
        }
        
        needs
    }
    
    /// Determine complexity level
    fn determine_complexity_level(agent_count: u32, _coordination_needs: &[String]) -> ComplexityLevel {
        match agent_count {
            1 => ComplexityLevel::Simple,
            2..=3 => ComplexityLevel::Moderate,
            4..=6 => ComplexityLevel::Complex,
            _ => ComplexityLevel::Enterprise,
        }
    }
    
    /// Check user quotas before agent creation
    fn check_user_quotas(user_principal: &str, requested_agents: u32) -> Result<QuotaCheckResult, String> {
        use crate::services::quota_manager::{QuotaManager, UserQuota, QuotaLimits, InferenceRate};
        
        // Get or create user quota
        let user_quota = with_state(|state| {
            state.user_quotas.get(user_principal).cloned()
        }).unwrap_or_else(|| {
            // Create default quota for new users (Pro tier)
            UserQuota {
                principal_id: user_principal.to_string(),
                subscription_tier: "Pro".to_string(),
                limits: QuotaLimits {
                    max_agents: 25,
                    monthly_agent_creations: 25,
                    token_limit: 4096,
                    inference_rate: InferenceRate::Priority,
                },
                current_usage: crate::services::quota_manager::QuotaUsage {
                    agents_created_this_month: 0,
                    tokens_used_this_month: 0,
                    inferences_this_month: 0,
                    last_reset_date: time(),
                },
                last_updated: time(),
            }
        });
        
        // Check if user has enough quota
        let current_agents = user_quota.current_usage.agents_created_this_month;
        let remaining_agents = user_quota.limits.max_agents.saturating_sub(current_agents);
        let quota_available = remaining_agents >= requested_agents && 
                             current_agents < user_quota.limits.monthly_agent_creations;
        
        // Store updated quota
        with_state_mut(|state| {
            state.user_quotas.insert(user_principal.to_string(), user_quota.clone());
        });
        
        Ok(QuotaCheckResult {
            quota_available,
            remaining_agents,
            monthly_limit: user_quota.limits.monthly_agent_creations,
            tier: user_quota.subscription_tier,
        })
    }
    
    /// Generate agent specifications based on parsed requirements
    fn generate_agent_specs(parsed: &ParsedRequirements) -> Result<Vec<AgentSpec>, String> {
        let mut specs = Vec::new();
        
        // Create specialized agents based on capabilities
        for (i, specialization) in parsed.specializations.iter().enumerate() {
            if i >= parsed.agent_count as usize {
                break;
            }
            
            let capabilities = Self::get_capabilities_for_specialization(specialization);
            let models = Self::get_models_for_specialization(specialization);
            
            specs.push(AgentSpec {
                agent_type: specialization.clone(),
                required_capabilities: capabilities,
                model_requirements: models,
                specialization: specialization.clone(),
            });
        }
        
        // If we need more agents than specializations, create generalist agents
        while specs.len() < parsed.agent_count as usize {
            specs.push(AgentSpec {
                agent_type: format!("Generalist Agent {}", specs.len() + 1),
                required_capabilities: vec!["general_assistance".to_string()],
                model_requirements: vec!["llama".to_string()],
                specialization: "General Assistant".to_string(),
            });
        }
        
        Ok(specs)
    }
    
    /// Get capabilities for a specific specialization
    fn get_capabilities_for_specialization(specialization: &str) -> Vec<String> {
        match specialization {
            "Software Developer" => vec!["coding", "software_development", "programming", "debugging"],
            "Test Engineer" => vec!["testing", "quality_assurance", "verification", "automation"],
            "Code Reviewer" => vec!["code_review", "quality_assurance", "best_practices", "security"],
            "Content Creator" => vec!["content_creation", "writing", "documentation", "editing"],
            "Marketing Specialist" => vec!["marketing", "social_media", "campaign_management", "analytics"],
            "Data Analyst" => vec!["data_analysis", "analytics", "reporting", "visualization"],
            "Research Analyst" => vec!["research", "investigation", "analysis", "synthesis"],
            _ => vec!["general_assistance"],
        }.into_iter().map(|s| s.to_string()).collect()
    }
    
    /// Get model suggestions for a specific specialization
    fn get_models_for_specialization(specialization: &str) -> Vec<String> {
        match specialization {
            "Software Developer" | "Test Engineer" | "Code Reviewer" => {
                vec!["code-llama", "starcoder", "wizardcoder"]
            },
            "Content Creator" | "Marketing Specialist" => {
                vec!["llama", "mistral", "gemma"]
            },
            "Data Analyst" | "Research Analyst" => {
                vec!["llama", "mistral", "gemma"]
            },
            _ => vec!["llama"],
        }.into_iter().map(|s| s.to_string()).collect()
    }
    
    /// Create coordination plan for multiple agents
    fn create_coordination_plan(parsed: &ParsedRequirements, agents: &[AgentSpec]) -> Result<String, String> {
        let mut plan = String::new();
        
        plan.push_str("Coordination Plan:\n");
        plan.push_str(&format!("- Total Agents: {}\n", agents.len()));
        plan.push_str(&format!("- Complexity Level: {:?}\n", parsed.complexity_level));
        
        if agents.len() > 1 {
            plan.push_str("- Coordination Strategy:\n");
            plan.push_str("  * Inter-agent communication enabled\n");
            plan.push_str("  * Task distribution based on specializations\n");
            plan.push_str("  * Progress tracking and synchronization\n");
            
            if !parsed.coordination_needs.is_empty() {
                plan.push_str("- Additional Coordination Needs:\n");
                for need in &parsed.coordination_needs {
                    plan.push_str(&format!("  * {}\n", need));
                }
            }
        }
        
        plan.push_str("- Agent Specializations:\n");
        for agent in agents {
            plan.push_str(&format!("  * {}: {}\n", agent.agent_type, agent.specialization));
        }
        
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_instructions_development() {
        let instructions = "Create a web application with React and Node.js backend";
        let parsed = InstructionAnalyzerService::parse_instructions(instructions).unwrap();
        
        assert!(parsed.required_capabilities.contains(&"coding".to_string()));
        assert!(parsed.required_capabilities.contains(&"software_development".to_string()));
        assert!(parsed.specializations.contains(&"Software Developer".to_string()));
        assert!(parsed.agent_count >= 1);
    }

    #[test]
    fn test_parse_instructions_content_creation() {
        let instructions = "Write a blog post about AI trends and create social media content";
        let parsed = InstructionAnalyzerService::parse_instructions(instructions).unwrap();
        
        assert!(parsed.required_capabilities.contains(&"content_creation".to_string()));
        assert!(parsed.required_capabilities.contains(&"writing".to_string()));
        assert!(parsed.specializations.contains(&"Content Creator".to_string()));
        assert!(parsed.agent_count >= 1);
    }

    #[test]
    fn test_parse_instructions_complex_team() {
        let instructions = "Build a complex software system with a team of developers, testers, and reviewers";
        let parsed = InstructionAnalyzerService::parse_instructions(instructions).unwrap();
        
        assert!(parsed.agent_count >= 3);
        assert!(parsed.complexity_level == ComplexityLevel::Complex || parsed.complexity_level == ComplexityLevel::Enterprise);
        assert!(!parsed.coordination_needs.is_empty());
    }

    #[test]
    fn test_generate_agent_specs() {
        let parsed = ParsedRequirements {
            agent_count: 2,
            required_capabilities: vec!["coding".to_string(), "testing".to_string()],
            model_requirements: vec!["code-llama".to_string()],
            specializations: vec!["Software Developer".to_string(), "Test Engineer".to_string()],
            coordination_needs: vec!["inter_agent_communication".to_string()],
            complexity_level: ComplexityLevel::Moderate,
        };
        
        let specs = InstructionAnalyzerService::generate_agent_specs(&parsed).unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].agent_type, "Software Developer");
        assert_eq!(specs[1].agent_type, "Test Engineer");
    }
}
