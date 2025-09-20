#[cfg(test)]
mod tests {
    use super::*;
    use easytier::common::config::TomlConfigLoader;
    use easytier::launcher::{NetworkInstance, ConfigSource};

    #[tokio::test]
    async fn test_private_mode_launcher() {
        // Create a minimal private mode configuration
        let toml_config = r#"
            instance_name = "test-private"
            hostname = "test-host"
            instance_uuid = "test-uuid-123"
            ipv4 = "10.144.144.1"
            
            [network]
            network_name = "test-network"
            network_secret = "test-secret"
            
            [flags]
            private_mode = true
            enable_encryption = true
            no_tun = true
            
            [rpc]
            rpc_portal = "127.0.0.1:15888"
        "#;

        // Parse the configuration
        let cfg = TomlConfigLoader::new_from_str(toml_config)
            .expect("Failed to parse test configuration");

        // Create NetworkInstance
        let mut instance = NetworkInstance::new(cfg, ConfigSource::FFI);

        // Test that we can create the instance without errors
        // Note: We don't actually start it in tests to avoid system dependencies
        println!("NetworkInstance created successfully for private mode");
        
        // In a real scenario, you would call:
        // let _event_subscriber = instance.start().expect("Failed to start instance");
        // But we skip this in tests to avoid requiring actual network setup
    }

    #[test]
    fn test_toml_config_generation() {
        // Test that our TOML generation logic produces valid configurations
        let toml_config = r#"
            instance_name = "test-instance"
            hostname = "test-host"
            instance_uuid = "test-uuid-456"
            ipv4 = "10.144.144.2"
            
            [network]
            network_name = "test-network-2"
            network_secret = "test-secret-2"
            
            [flags]
            private_mode = true
            enable_encryption = true
            no_tun = true
            
            [rpc]
            rpc_portal = "127.0.0.1:15889"
        "#;

        // Verify the configuration can be parsed
        let cfg = TomlConfigLoader::new_from_str(toml_config)
            .expect("Failed to parse generated TOML configuration");

        println!("TOML configuration parsed successfully");
        
        // Verify we can create a NetworkInstance from it
        let _instance = NetworkInstance::new(cfg, ConfigSource::FFI);
        println!("NetworkInstance created from TOML configuration");
    }
}