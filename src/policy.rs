use crate::config::PolicyConfig;
use anyhow::{Context, Result};
use colored::Colorize;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Allow,
    Block,
    RequireApproval,
    Mask,
}

impl From<&str> for Action {
    fn from(s: &str) -> Self {
        match s {
            "block" => Action::Block,
            "require_approval" => Action::RequireApproval,
            "mask" => Action::Mask,
            _ => Action::Allow,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Decision {
    pub allowed: bool,
    pub action: Action,
    pub violated_policy: Option<String>,
    pub reason: Option<String>,
    pub require_approval: bool,
    pub approval_prompt: Option<String>,
}

#[derive(Debug)]
pub struct CompiledPolicy {
    pub id: String,
    pub name: String,
    pub target_tools: HashSet<String>,
    pub match_all_tools: bool,
    pub action: Action,
    pub compiled_matches: HashMap<String, Vec<Regex>>,
    pub message: String,
}

#[derive(Debug)]
pub struct PolicyEngine {
    policies: Vec<CompiledPolicy>,
}

impl PolicyEngine {
    pub fn new(configs: &[PolicyConfig]) -> Result<Self> {
        let mut compiled = Vec::with_capacity(configs.len());

        for cfg in configs {
            let mut target_tools = HashSet::new();
            let mut match_all_tools = false;

            for tool in &cfg.target_tools {
                if tool == "*" {
                    match_all_tools = true;
                } else {
                    target_tools.insert(tool.clone());
                }
            }

            let mut compiled_matches = HashMap::new();
            for (param, patterns) in &cfg.conditions.param_matches {
                let mut regexes = Vec::with_capacity(patterns.len());
                for pat in patterns {
                    let re = Regex::new(pat)
                        .with_context(|| format!("Invalid regex '{}' in policy '{}'", pat, cfg.id))?;
                    regexes.push(re);
                }
                compiled_matches.insert(param.clone(), regexes);
            }

            compiled.push(CompiledPolicy {
                id: cfg.id.clone(),
                name: cfg.name.clone(),
                target_tools,
                match_all_tools,
                action: Action::from(cfg.action.as_str()),
                compiled_matches,
                message: cfg.message.clone(),
            });
        }

        Ok(Self { policies: compiled })
    }

    pub fn evaluate(&self, tool_name: &str, args: &HashMap<String, Value>) -> Decision {
        for p in &self.policies {
            // 1. Tool name match
            if !p.match_all_tools && !p.target_tools.contains(tool_name) {
                continue;
            }

            // 2. Check parameter regex conditions
            let mut matched = false;
            for (param_name, regex_list) in &p.compiled_matches {
                if let Some(val) = args.get(param_name) {
                    let val_str = match val {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };

                    for re in regex_list {
                        if re.is_match(&val_str) {
                            matched = true;
                            break;
                        }
                    }
                }
                if matched {
                    break;
                }
            }

            // Trigger policy if condition matched or if policy applies to all calls of target tool
            if matched || p.compiled_matches.is_empty() {
                match p.action {
                    Action::Block => {
                        return Decision {
                            allowed: false,
                            action: Action::Block,
                            violated_policy: Some(p.id.clone()),
                            reason: Some(p.message.clone()),
                            require_approval: false,
                            approval_prompt: None,
                        };
                    }
                    Action::RequireApproval => {
                        return Decision {
                            allowed: false,
                            action: Action::RequireApproval,
                            violated_policy: Some(p.id.clone()),
                            reason: Some(p.message.clone()),
                            require_approval: true,
                            approval_prompt: Some(format!(
                                "Agent requested tool '{}' with args {:?}. Policy: {}",
                                tool_name, args, p.message
                            )),
                        };
                    }
                    _ => {}
                }
            }
        }

        Decision {
            allowed: true,
            action: Action::Allow,
            violated_policy: None,
            reason: None,
            require_approval: false,
            approval_prompt: None,
        }
    }
}

/// Requests human approval in the terminal with an asynchronous timeout.
pub async fn request_terminal_approval(prompt: &str, timeout_dur: Duration) -> bool {
    println!("\n{}", "[PROPYLON APPROVAL REQUIRED]".bold().yellow());
    println!("{}", prompt);
    print!("{}", "Do you approve this action? (y/N): ".bold().cyan());
    let _ = io::stdout().flush();

    let read_input = tokio::task::spawn_blocking(|| {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let trimmed = input.trim().to_lowercase();
                trimmed == "y" || trimmed == "yes"
            }
            Err(_) => false,
        }
    });

    match timeout(timeout_dur, read_input).await {
        Ok(Ok(approved)) => approved,
        Ok(Err(_)) => false,
        Err(_) => {
            println!(
                "\n{}",
                "[PROPYLON TIMEOUT] Approval timed out. Action rejected by default."
                    .bold()
                    .red()
            );
            false
        }
    }
}
