use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: String,
    pub server: ServerConfig,
    #[serde(default)]
    pub audit: AuditConfig,
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
    #[serde(default)]
    pub webhook: WebhookConfig,
    #[serde(default)]
    pub upstreams: Vec<UpstreamConfig>,
    #[serde(default)]
    pub policies: Vec<PolicyConfig>,
}

fn default_version() -> String {
    "v1".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_addr")]
    pub addr: String,
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_addr() -> String {
    "0.0.0.0:8080".to_string()
}

fn default_mode() -> String {
    "http".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_output")]
    pub output: String,
    #[serde(default)]
    pub log_arguments: bool,
    #[serde(default)]
    pub mask_sensitive_data: bool,
}

fn default_output() -> String {
    "stdout".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_calls")]
    pub max_calls_per_minute: u32,
    #[serde(default = "default_loop_threshold")]
    pub loop_threshold: u32,
    #[serde(default = "default_loop_window")]
    pub loop_window_seconds: u64,
}

fn default_true() -> bool {
    true
}

fn default_max_calls() -> u32 {
    60
}

fn default_loop_threshold() -> u32 {
    5
}

fn default_loop_window() -> u64 {
    15
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_calls_per_minute: default_max_calls(),
            loop_threshold: default_loop_threshold(),
            loop_window_seconds: default_loop_window(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: Option<String>,
    #[serde(default = "default_webhook_timeout")]
    pub timeout_seconds: u64,
}

fn default_webhook_timeout() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub upstream_type: String,
    pub target: Option<String>,
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub id: String,
    pub name: String,
    pub target_tools: Vec<String>,
    pub action: String, // "block", "allow", "require_approval", "mask"
    #[serde(default)]
    pub conditions: PolicyConditions,
    pub approval_channel: Option<String>,
    pub timeout: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyConditions {
    #[serde(default)]
    pub param_matches: HashMap<String, Vec<String>>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse YAML configuration")?;
        Ok(config)
    }
}
