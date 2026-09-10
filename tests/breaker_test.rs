use propylon::breaker::{BreakerError, CircuitBreaker};
use propylon::config::CircuitBreakerConfig;
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_circuit_breaker_loop_detection() {
    let cfg = CircuitBreakerConfig {
        enabled: true,
        max_calls_per_minute: 100,
        loop_threshold: 3, // Trip on 4th identical call
        loop_window_seconds: 10,
    };

    let breaker = CircuitBreaker::new(cfg);

    let mut args = HashMap::new();
    args.insert("query".to_string(), json!("SELECT * FROM users"));

    // First 3 calls should pass
    assert!(breaker.check("query_db", &args).is_ok());
    assert!(breaker.check("query_db", &args).is_ok());
    assert!(breaker.check("query_db", &args).is_ok());

    // 4th identical call must trip the loop circuit breaker
    let err = breaker.check("query_db", &args).unwrap_err();
    match err {
        BreakerError::LoopDetected { count, window_secs } => {
            assert_eq!(count, 4);
            assert_eq!(window_secs, 10);
        }
        _ => panic!("Expected LoopDetected error"),
    }
}

#[test]
fn test_circuit_breaker_distinct_calls_pass() {
    let cfg = CircuitBreakerConfig {
        enabled: true,
        max_calls_per_minute: 100,
        loop_threshold: 3,
        loop_window_seconds: 10,
    };

    let breaker = CircuitBreaker::new(cfg);

    for i in 0..5 {
        let mut args = HashMap::new();
        args.insert("query".to_string(), json!(format!("SELECT * FROM table_{}", i)));
        assert!(breaker.check("query_db", &args).is_ok());
    }
}

#[test]
fn test_circuit_breaker_rate_limiting() {
    let cfg = CircuitBreakerConfig {
        enabled: true,
        max_calls_per_minute: 5,
        loop_threshold: 100, // loop detection disabled for this test
        loop_window_seconds: 10,
    };

    let breaker = CircuitBreaker::new(cfg);

    for i in 0..5 {
        let mut args = HashMap::new();
        args.insert("id".to_string(), json!(i));
        assert!(breaker.check("get_item", &args).is_ok());
    }

    // 6th call should be rate-limited
    let mut args = HashMap::new();
    args.insert("id".to_string(), json!(999));
    let err = breaker.check("get_item", &args).unwrap_err();
    match err {
        BreakerError::RateLimited { current, max } => {
            assert_eq!(current, 5);
            assert_eq!(max, 5);
        }
        _ => panic!("Expected RateLimited error"),
    }
}
