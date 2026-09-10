use crate::breaker::CircuitBreaker;
use crate::config::Config;
use crate::policy::PolicyEngine;
use anyhow::Result;
use arc_swap::ArcSwap;
use colored::Colorize;
use notify::{Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct ConfigWatcher;

impl ConfigWatcher {
    pub fn spawn_watcher(
        config_path: PathBuf,
        shared_engine: Arc<ArcSwap<PolicyEngine>>,
        shared_breaker: Arc<ArcSwap<CircuitBreaker>>,
    ) -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<notify::Result<Event>>(100);

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            NotifyConfig::default(),
        )?;

        // Watch the file or its parent directory
        let watch_target = if config_path.is_file() {
            config_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
        } else {
            config_path.clone()
        };

        watcher.watch(&watch_target, RecursiveMode::NonRecursive)?;

        tokio::spawn(async move {
            // Keep watcher alive inside this task
            let _watcher = watcher;
            info!(
                path = %config_path.display(),
                "Started zero-downtime configuration watcher"
            );

            while let Some(res) = rx.recv().await {
                match res {
                    Ok(event) => {
                        if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                            // Debounce editor multi-writes
                            tokio::time::sleep(Duration::from_millis(150)).await;

                            // Drain queued duplicate events
                            while rx.try_recv().is_ok() {}

                            match Config::load(&config_path) {
                                Ok(new_cfg) => {
                                    match PolicyEngine::new(&new_cfg.policies) {
                                        Ok(new_engine) => {
                                            let count = new_cfg.policies.len();
                                            shared_engine.store(Arc::new(new_engine));
                                            shared_breaker.store(Arc::new(CircuitBreaker::new(new_cfg.circuit_breaker)));
                                            println!(
                                                "\n{} {} ({} policies active)",
                                                "[Hot-Reload]".bold().green(),
                                                "Successfully reloaded configuration with ZERO downtime!".green(),
                                                count
                                            );
                                            info!(policies = count, "Configuration hot-reloaded successfully");
                                        }
                                        Err(err) => {
                                            error!(error = %err, "Failed to compile new policies during hot-reload");
                                            eprintln!(
                                                "\n{} Policy compilation error: {}. Existing rules remain active.",
                                                "[Hot-Reload Rejected]".bold().red(),
                                                err
                                            );
                                        }
                                    }
                                }
                                Err(err) => {
                                    warn!(error = %err, "Failed to parse modified config file");
                                    eprintln!(
                                        "\n{} Invalid YAML syntax: {}. Existing rules remain active.",
                                        "[Hot-Reload Rejected]".bold().yellow(),
                                        err
                                    );
                                }
                            }
                        }
                    }
                    Err(err) => {
                        warn!(error = %err, "Filesystem watch error");
                    }
                }
            }
        });

        Ok(())
    }
}
