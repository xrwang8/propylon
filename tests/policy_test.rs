use propylon::config::{PolicyConditions, PolicyConfig};
use propylon::policy::{Action, PolicyEngine};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_rust_policy_engine_block() {
    let mut param_matches = HashMap::new();
    param_matches.insert(
        "command".to_string(),
        vec![r"(?i)rm\s+-rf\s+/".to_string()],
    );

    let configs = vec![PolicyConfig {
        id: "block-dangerous-bash".to_string(),
        name: "Block Destructive Commands".to_string(),
        target_tools: vec!["execute_command".to_string()],
        action: "block".to_string(),
        conditions: PolicyConditions { param_matches },
        approval_channel: None,
        timeout: None,
        message: "Destructive command blocked.".to_string(),
    }];

    let engine = PolicyEngine::new(&configs).expect("Failed to initialize engine");

    // 1. Dangerous command must be blocked
    let mut dangerous_args = HashMap::new();
    dangerous_args.insert(
        "command".to_string(),
        json!("rm -rf / --no-preserve-root"),
    );

    let decision = engine.evaluate("execute_command", &dangerous_args);
    assert!(!decision.allowed);
    assert_eq!(decision.action, Action::Block);
    assert_eq!(decision.violated_policy.as_deref(), Some("block-dangerous-bash"));

    // 2. Safe command must be allowed
    let mut safe_args = HashMap::new();
    safe_args.insert("command".to_string(), json!("ls -la /var/log"));

    let decision_safe = engine.evaluate("execute_command", &safe_args);
    assert!(decision_safe.allowed);
    assert_eq!(decision_safe.action, Action::Allow);
}
