use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use clap::Parser;
use colored::Colorize;
use propylon::{
    config::Config,
    policy::{request_terminal_approval, PolicyEngine},
    protocol::{
        CallToolParams, CallToolResult, Request, Response, ToolContent, CODE_APPROVAL_DENIED,
        CODE_INVALID_PARAMS, CODE_PARSE_ERROR, CODE_POLICY_VIOLATION,
    },
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const BANNER: &str = r#"
  ____                               _               
 |  _ \ _ __ ___  _ __  _   _| | ___  _ __  
 | |_) | '__/ _ \| '_ \| | | | |/ _ \| '_ \ 
 |  __/| | | (_) | |_) | |_| | | (_) | | | |
 |_|   |_|  \___/| .__/ \__, |_|\___/|_| |_|
                 |_|    |___/               
    The Core Security Gateway & Firewall for AI Agents
    Engine: Memory-Safe Rust | Origin: Προπύλαιον
------------------------------------------------------------
"#;

#[derive(Parser, Debug)]
#[command(name = "propylon", version = VERSION, about = "The Monumental Security Gateway for AI Agents")]
struct Args {
    #[arg(short, long, default_value = "configs/propylon.example.yaml", help = "Path to configuration file")]
    config: String,

    #[arg(short, long, help = "Override server bind address (e.g. 0.0.0.0:8080)")]
    addr: Option<String>,
}

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    policy_engine: Arc<PolicyEngine>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "propylon=info,tower_http=info".into()),
        )
        .init();

    let args = Args::parse();
    println!("{}", BANNER.cyan());
    println!("  Starting Propylon v{} in memory-safe Rust...\n", VERSION);

    // 2. Load Configuration
    info!("Loading configuration from: {}", args.config);
    let cfg = Config::load(&args.config)?;
    let bind_addr = args.addr.unwrap_or_else(|| cfg.server.addr.clone());

    // 3. Initialize Policy Engine
    info!("Compiling {} security policies...", cfg.policies.len());
    let policy_engine = PolicyEngine::new(&cfg.policies)?;

    let state = AppState {
        config: Arc::new(cfg),
        policy_engine: Arc::new(policy_engine),
    };

    // 4. Build Axum HTTP Router
    let app = Router::new()
        .route("/healthz", get(health_check))
        .route("/v1/mcp", post(handle_mcp))
        .route("/", post(handle_mcp))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = bind_addr.parse().expect("Invalid server bind address");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Propylon Gateway listening on http://{}", addr);
    println!(
        "{} Propylon Gateway is actively guarding AI tool invocations.\n",
        "✓".green().bold()
    );

    // 5. Run Server with Graceful Shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Propylon Gateway shutdown complete. Farewell.");
    Ok(())
}

async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "gateway": "Propylon",
            "engine": "rust",
            "version": VERSION
        })),
    )
}

async fn handle_mcp(
    State(state): State<AppState>,
    Json(payload): Json<Request>,
) -> impl IntoResponse {
    let req_id = payload.id.clone();

    match payload.method.as_str() {
        "tools/call" => {
            let params_val = match payload.params {
                Some(p) => p,
                None => {
                    return Json(Response::error(
                        req_id,
                        CODE_INVALID_PARAMS,
                        "Missing params for tools/call",
                    ));
                }
            };

            let call_params: CallToolParams = match serde_json::from_value(params_val) {
                Ok(p) => p,
                Err(e) => {
                    return Json(Response::error(
                        req_id,
                        CODE_PARSE_ERROR,
                        format!("Invalid CallToolParams: {}", e),
                    ));
                }
            };

            if state.config.audit.enabled {
                info!(
                    tool = %call_params.name,
                    args = ?call_params.arguments,
                    "[Audit] Inspecting incoming Tool Call"
                );
            }

            // Policy Evaluation
            let decision = state
                .policy_engine
                .evaluate(&call_params.name, &call_params.arguments);

            if !decision.allowed {
                if decision.require_approval {
                    let prompt = decision.approval_prompt.unwrap_or_else(|| {
                        format!("Execution of tool '{}' requires confirmation.", call_params.name)
                    });

                    let approved = request_terminal_approval(&prompt, Duration::from_secs(30)).await;
                    if !approved {
                        warn!(tool = %call_params.name, "REJECTED by operator approval");
                        return Json(Response::error(
                            req_id,
                            CODE_APPROVAL_DENIED,
                            "Action rejected by security supervisor",
                        ));
                    }
                    info!(tool = %call_params.name, "APPROVED by operator");
                } else {
                    let reason = decision.reason.unwrap_or_else(|| "Security policy violation".into());
                    let policy_id = decision.violated_policy.unwrap_or_default();
                    error!(
                        tool = %call_params.name,
                        policy = %policy_id,
                        reason = %reason,
                        "BLOCKED tool call"
                    );
                    return Json(Response::error(
                        req_id,
                        CODE_POLICY_VIOLATION,
                        format!("Blocked by Propylon policy '{}': {}", policy_id, reason),
                    ));
                }
            }

            info!(tool = %call_params.name, "PASSED tool call inspection");
            let result = CallToolResult {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: Some(format!(
                        "[Propylon Verified (Rust)] Successfully validated and forwarded tool '{}'",
                        call_params.name
                    )),
                }],
                is_error: Some(false),
            };

            Json(Response::success(
                req_id,
                serde_json::to_value(result).unwrap(),
            ))
        }
        _ => {
            // Passthrough for tools/list, etc.
            Json(Response::success(
                req_id,
                json!({
                    "message": "Propylon Gateway pass-through (Rust)",
                    "method": payload.method
                }),
            ))
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\n{}", "Received shutdown signal. Closing connections...".yellow());
}
