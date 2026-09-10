use crate::config::CircuitBreakerConfig;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
pub enum BreakerError {
    RateLimited { current: u32, max: u32 },
    LoopDetected { count: u32, window_secs: u64 },
}

impl std::fmt::Display for BreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakerError::RateLimited { current, max } => {
                write!(
                    f,
                    "Rate limit exceeded: {} tool calls in the last minute (max: {})",
                    current, max
                )
            }
            BreakerError::LoopDetected { count, window_secs } => {
                write!(
                    f,
                    "Agent loop detected: {} identical tool calls within {}s. Execution halted by circuit breaker.",
                    count, window_secs
                )
            }
        }
    }
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Mutex<BreakerState>,
}

struct BreakerState {
    call_timestamps: VecDeque<Instant>,
    call_history: VecDeque<(String, Instant)>, // (call_signature, timestamp)
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(BreakerState {
                call_timestamps: VecDeque::new(),
                call_history: VecDeque::new(),
            }),
        }
    }

    pub fn check(&self, tool_name: &str, args: &HashMap<String, Value>) -> Result<(), BreakerError> {
        if !self.config.enabled {
            return Ok(());
        }

        let now = Instant::now();
        let mut state = self.state.lock().unwrap();

        // 1. Prune timestamps older than 60s
        let one_minute_ago = now.checked_sub(Duration::from_secs(60)).unwrap_or(now);
        while let Some(&t) = state.call_timestamps.front() {
            if t < one_minute_ago {
                state.call_timestamps.pop_front();
            } else {
                break;
            }
        }

        // Check rate limit (calls per minute)
        let current_calls = state.call_timestamps.len() as u32;
        if current_calls >= self.config.max_calls_per_minute {
            return Err(BreakerError::RateLimited {
                current: current_calls,
                max: self.config.max_calls_per_minute,
            });
        }

        // 2. Prune loop history older than loop_window_seconds
        let loop_window = Duration::from_secs(self.config.loop_window_seconds);
        let loop_cutoff = now.checked_sub(loop_window).unwrap_or(now);
        while let Some((_, t)) = state.call_history.front() {
            if *t < loop_cutoff {
                state.call_history.pop_front();
            } else {
                break;
            }
        }

        // Generate call signature (tool_name + sorted arguments)
        let signature = Self::compute_signature(tool_name, args);

        // Count identical calls in current window
        let identical_count = state
            .call_history
            .iter()
            .filter(|(sig, _)| sig == &signature)
            .count() as u32;

        if identical_count >= self.config.loop_threshold {
            return Err(BreakerError::LoopDetected {
                count: identical_count + 1,
                window_secs: self.config.loop_window_seconds,
            });
        }

        // Record current call
        state.call_timestamps.push_back(now);
        state.call_history.push_back((signature, now));

        Ok(())
    }

    fn compute_signature(tool_name: &str, args: &HashMap<String, Value>) -> String {
        // Deterministic serialized argument representation
        let mut sorted_keys: Vec<&String> = args.keys().collect();
        sorted_keys.sort();

        let mut sig = format!("tool:{}|", tool_name);
        for k in sorted_keys {
            if let Some(v) = args.get(k) {
                sig.push_str(&format!("{}:{}|", k, v));
            }
        }
        sig
    }
}
