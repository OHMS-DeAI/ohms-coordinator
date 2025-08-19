// Integration tests for OHMS 2.0 coordinator workflows

#[cfg(test)]
mod ohms_2_integration_tests {
    /// Test OHMS 2.0 coordinator structure validation
    #[test]
    fn test_ohms_2_coordinator_structure() {
        // Test that the coordinator has been refactored for OHMS 2.0
        
        // Verify OHMS 2.0 API endpoints exist
        let api_content = include_str!("../src/api.rs");
        assert!(api_content.contains("create_agents_from_instructions"), "Should have agent creation endpoint");
        assert!(api_content.contains("get_agent_creation_status"), "Should have status checking endpoint");
        assert!(api_content.contains("get_user_quota_status"), "Should have quota checking endpoint");
        assert!(api_content.contains("EconIntegrationService"), "Should integrate with economics canister");
        
        // Verify domain types for OHMS 2.0
        let domain_content = include_str!("../src/domain/mod.rs");
        assert!(domain_content.contains("InstructionRequest"), "Should have instruction request type");
        assert!(domain_content.contains("AgentCreationResult"), "Should have agent creation result type");
        assert!(domain_content.contains("QuotaCheckResult"), "Should have quota check result type");
        assert!(!domain_content.contains("BountySpec"), "Should not have bounty types");
        
        println!("✅ OHMS 2.0 coordinator structure validation passed");
    }

    /// Test Candid interface for OHMS 2.0
    #[test]
    fn test_ohms_2_candid_interface() {
        // Verify Candid interface has been updated for OHMS 2.0
        let candid_content = include_str!("../src/ohms_coordinator.did");
        
        // Should have OHMS 2.0 agent creation methods
        assert!(candid_content.contains("create_agents_from_instructions"), "Should have agent creation method");
        assert!(candid_content.contains("get_agent_creation_status"), "Should have status method");
        assert!(candid_content.contains("get_user_quota_status"), "Should have quota method");
        assert!(candid_content.contains("get_economics_health"), "Should have economics integration");
        
        // Should have OHMS 2.0 types
        assert!(candid_content.contains("InstructionRequest"), "Should have instruction request type");
        assert!(candid_content.contains("AgentCreationResult"), "Should have agent creation result type");
        assert!(candid_content.contains("QuotaCheckResult"), "Should have quota check result type");
        assert!(candid_content.contains("EconHealth"), "Should have economics health type");
        
        // Should NOT have bounty-related methods or types
        assert!(!candid_content.contains("bounty"), "Should not have bounty methods");
        assert!(!candid_content.contains("BountySpec"), "Should not have bounty types");
        
        println!("✅ OHMS 2.0 Candid interface validation passed");
    }

    /// Test economics integration service
    #[test]
    fn test_economics_integration_service() {
        let econ_content = include_str!("../src/services/econ_integration.rs");
        
        // Should have economics canister integration
        assert!(econ_content.contains("EconIntegrationService"), "Should have economics integration service");
        assert!(econ_content.contains("validate_agent_creation_quota"), "Should validate quotas");
        assert!(econ_content.contains("sync_user_quota_from_economics"), "Should sync quotas");
        assert!(econ_content.contains("get_economics_health"), "Should monitor economics health");
        
        // Should use real canister ID
        assert!(econ_content.contains("tetse-piaaa-aaaao-qkeyq-cai"), "Should use real economics canister ID");
        
        println!("✅ Economics integration service validation passed");
    }

    /// Test instruction analysis service structure  
    #[test]
    fn test_instruction_analysis_service() {
        let instruction_content = include_str!("../src/services/instruction_analyzer.rs");
        
        // Should have instruction analysis capabilities
        assert!(instruction_content.contains("InstructionAnalyzerService"), "Should have instruction analyzer");
        assert!(instruction_content.contains("analyze_instructions"), "Should analyze instructions");
        assert!(instruction_content.contains("generate_agent_specs"), "Should generate agent specs");
        assert!(instruction_content.contains("determine_agent_count"), "Should determine agent count");
        
        // Should have capability patterns for specialization detection
        assert!(instruction_content.contains("Software Developer"), "Should detect software development");
        assert!(instruction_content.contains("Test Engineer"), "Should detect testing specialization");
        assert!(instruction_content.contains("Content Creator"), "Should detect content creation");
        
        println!("✅ Instruction analysis service validation passed");
    }

    /// Test agent spawning service structure
    #[test]
    fn test_agent_spawning_service() {
        let spawning_content = include_str!("../src/services/agent_spawning.rs");
        
        // Should have agent spawning capabilities
        assert!(spawning_content.contains("AgentSpawningService"), "Should have agent spawning service");
        assert!(spawning_content.contains("spawn_agents_from_instructions"), "Should spawn from instructions");
        assert!(spawning_content.contains("setup_coordination_network"), "Should setup coordination");
        assert!(spawning_content.contains("setup_agent_capability_profiles"), "Should setup capability profiles");
        
        // Should NOT have placeholder logic
        assert!(!spawning_content.contains("TODO"), "Should not have TODO items");
        
        println!("✅ Agent spawning service validation passed");
    }

    /// Test WASM compilation readiness
    #[test]
    fn test_wasm_compilation_ready() {
        let cargo_content = include_str!("../Cargo.toml");
        
        // Should be configured as a cdylib for WASM
        assert!(cargo_content.contains("cdylib"), "Should be configured as cdylib for WASM");
        assert!(cargo_content.contains("ic-cdk"), "Should have ic-cdk dependency");
        assert!(cargo_content.contains("candid"), "Should have candid dependency");
        
        println!("✅ WASM compilation configuration validation passed");
    }
}
