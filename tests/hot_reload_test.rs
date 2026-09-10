use arc_swap::ArcSwap;
use propylon::config::Config;
use propylon::policy::PolicyEngine;
use std::fs;
use std::sync::Arc;

#[test]
fn test_atomic_policy_swap() {
    let yaml_v1 = r#"
version: "v1"
server:
  addr: "127.0.0.1:8080"
policies:
  - id: "policy-1"
    name: "Initial Rule"
    target_tools: ["test_tool"]
    action: "block"
    message: "Blocked by initial policy"
"#;

    let yaml_v2 = r#"
version: "v1"
server:
  addr: "127.0.0.1:8080"
policies:
  - id: "policy-1"
    name: "Initial Rule"
    target_tools: ["test_tool"]
    action: "block"
    message: "Blocked by initial policy"
  - id: "policy-2"
    name: "Dynamically Added Rule"
    target_tools: ["new_tool"]
    action: "block"
    message: "Blocked by dynamic hot-reload"
"#;

    let temp_dir = std::env::temp_dir().join("propylon_hot_reload_test");
    let _ = fs::create_dir_all(&temp_dir);
    let config_path = temp_dir.join("propylon.yaml");

    // 1. Initial configuration
    fs::write(&config_path, yaml_v1).unwrap();
    let cfg1 = Config::load(&config_path).unwrap();
    assert_eq!(cfg1.policies.len(), 1);

    let engine1 = PolicyEngine::new(&cfg1.policies).unwrap();
    let shared_engine = Arc::new(ArcSwap::from_pointee(engine1));

    // Initially new_tool is allowed
    let decision1 = shared_engine.load().evaluate("new_tool", &std::collections::HashMap::new());
    assert!(decision1.allowed);

    // 2. Simulate hot reload to v2
    fs::write(&config_path, yaml_v2).unwrap();
    let cfg2 = Config::load(&config_path).unwrap();
    assert_eq!(cfg2.policies.len(), 2);

    let engine2 = PolicyEngine::new(&cfg2.policies).unwrap();
    shared_engine.store(Arc::new(engine2));

    // Now new_tool must be blocked zero-downtime!
    let decision2 = shared_engine.load().evaluate("new_tool", &std::collections::HashMap::new());
    assert!(!decision2.allowed);
    assert_eq!(decision2.violated_policy.as_deref(), Some("policy-2"));

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
