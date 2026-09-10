<div align="center">

# 🏛️ Propylon
### The Monumental Security Gateway & WAF for Autonomous AI Agents

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust Version](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)
[![Status](https://img.shields.io/badge/Release-v0.2.0-brightgreen.svg)]()

**Don't let autonomous AI agents run wild in production.**  
*An ultra-fast, memory-safe Rust security gateway, proxy, and behavioral firewall for Model Context Protocol (MCP) and agentic tool invocations.*

[Quick Start](#-quick-start-in-60-seconds) • [Architecture](#-architecture) • [Key Features](#-key-features) • [Configuration](#-configuration) • [Roadmap](#-roadmap)

---

</div>

## 📖 The Origin: What is *Propylon*?

> In classical Greece, a **Propylon** (*Greek: Προπύλαιον*) was the monumental, fortified gateway structure flanking the entrance to a sanctuary, acropolis, or imperial citadel—most famously, the **Propylaea of the Acropolis of Athens**.
>
> All travelers, merchants, and envoys were required to pass through the Propylon: guards inspected their belongings, verified their credentials, and disarmed all weapons before granting passage into the sacred city.
>
> **Propylon** serves the exact same purpose for the era of Autonomous Agents: standing guard at the threshold between autonomous AI models (Claude, Cursor, AutoGen, CrewAI) and your mission-critical databases, clouds, and enterprise APIs.

---

## 💥 Why Propylon? The Production Dilemma

As developers, we are granting AI agents unprecedented power: bash shells, database access, cloud credentials, and internal APIs. But autonomous models are inherently probabilistic and vulnerable:

1. **Hallucination & Misalignment**: An agent trying to "clean up temporary files" runs `rm -rf /` or deletes active database tables.
2. **Agent Dead Loops**: When an agent fails to parse an output, it frequently enters an infinite loop, calling the same tool 100 times in seconds, exhausting tokens and crashing backends.
3. **Indirect Prompt Injection**: Malicious instructions embedded inside a web page or ticket instruct your agent to exfiltrate database records or AWS secrets.
4. **The Blind Execution Gap**: Enterprises cannot allow agents to execute state-altering operations without human oversight, yet building custom approval workflows into every agent is an architectural nightmare.

**Propylon solves this.** It acts as a transparent reverse-proxy and firewall between any AI Agent and your MCP servers / APIs, giving you total visibility, granular policy enforcement, and interactive human-in-the-loop control.

---

## 🏛️ Architecture

```mermaid
flowchart LR
    subgraph Agentic_Clients ["Client Layer"]
        A1[Claude Desktop]
        A2[Cursor / Cline]
        A3[Custom LangGraph / AutoGen]
    end

    subgraph Propylon_Gateway ["🏛️ Propylon Security Gateway (Rust / Tokio / Axum)"]
        direction TB
        P0[Circuit Breaker / Loop Detector]
        P1[Protocol Interceptor / JSON-RPC]
        P2{Lock-Free Policy Engine / ArcSwap}
        P3[Human-in-the-Loop: Terminal / Webhook]
        P4[DLP & Secret Masking]
        P5[Zero-Trust Audit Trail]
        W1[Zero-Downtime Hot-Reload Watcher]
    end

    subgraph Upstream_Tools ["Enterprise Tools & Backends"]
        T1[(PostgreSQL / MySQL)]
        T2[Terminal / Bash Shell]
        T3[Kubernetes / Cloud APIs]
        T4[GitHub / Jira / Slack]
    end

    Agentic_Clients -->|MCP / Tool Calls| P0
    P0 -->|Pass Loop & Rate Checks| P1
    P1 --> P2
    P2 -->|High Risk Action| P3
    P3 -->|Approved| P4
    P2 -->|Allowed Action| P4
    P2 -->|Blocked Action| P1
    P4 --> Upstream_Tools
    P4 --> P5
    W1 -.->|Atomic Swap| P2
```

---

## ✨ Key Features

- 🦀 **100% Memory-Safe Rust Core**: Engineered with Tokio, Axum, and ArcSwap for zero-cost abstractions, zero garbage collection pauses, and predictable sub-millisecond inspection latency.
- ⚡ **Zero-Downtime Policy Hot-Reloading**: Automatically detects modifications to your security configuration and hot-swaps policies atomically in memory without terminating long-lived SSE connections or dropping packets.
- 🔄 **Agent Loop & Rate-Limit Circuit Breaker**: Real-time sliding-window rate limiting and call signature hashing. Instantly halts hallucinating agents from entering rapid-fire infinite tool loops.
- 🛡️ **Zero-Trust Tool Firewalls**: Inspect tool parameters at the AST and regex level before execution. Block destructive shell commands (`rm -rf`, `mkfs`) and irreversible SQL (`DROP TABLE`, `TRUNCATE`).
- 🚦 **Multi-Channel Human-in-the-Loop (HITL)**: Automatically hold high-risk requests (e.g., financial transactions, production deployments) and request interactive approval in your terminal or via remote Webhooks (Slack, Feishu, DingTalk).
- 📦 **Single Static Binary (8.4MB)**: Compiles to a self-contained, dependency-free native binary. Drop it directly into Docker, Kubernetes pods, or developer workstations.
- 📜 **Tamper-Evident Audit Logging**: Comprehensive telemetry tracking every agent trajectory, tool invocation, argument hash, and outcome for post-mortem analysis and compliance.

---

## 🚀 Quick Start in 60 Seconds

### 1. Build and Run

```bash
# Clone the repository
git clone https://github.com/xrwang8/propylon.git
cd propylon

# Build and start the gateway using cargo / make
make run
```

You will see the gateway start up on `http://127.0.0.1:8080`:

```text
  ____                               _               
 |  _ \ _ __ ___  _ __  _   _| | ___  _ __  
 | |_) | '__/ _ \| '_ \| | | | |/ _ \| '_ \ 
 |  __/| | | (_) | |_) | |_| | | (_) | | | |
 |_|   |_|  \___/| .__/ \__, |_|\___/|_| |_|
                 |_|    |___/               
    The Core Security Gateway & Firewall for AI Agents
    Engine: Memory-Safe Rust | Origin: Προπύλαιον
------------------------------------------------------------
  Starting Propylon v0.2.0 in memory-safe Rust...

[INFO] Propylon Gateway listening on http://0.0.0.0:8080
✓ Propylon Gateway v0.2.0 is actively guarding AI tool invocations.
✓ Live policy hot-reloading & agent loop circuit breaker active.
```

### 2. Test Dangerous Command Interception

In another terminal, simulate an AI agent attempting a destructive command:

```bash
curl -X POST http://127.0.0.1:8080/v1/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "execute_command",
      "arguments": {
        "command": "rm -rf / --no-preserve-root"
      }
    }
  }'
```

**Propylon blocks the attempt immediately:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Blocked by Propylon policy 'block-dangerous-bash': Security Policy Violation: Destructive shell command detected."
  }
}
```

### 3. Test Infinite Loop Protection

If an agent goes rogue and spams identical tool calls:

```bash
for i in {1..6}; do
  curl -s -X POST http://127.0.0.1:8080/v1/mcp \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"query_db","arguments":{"query":"SELECT * FROM users"}}}'
done
```

**The Circuit Breaker trips and halts the loop:**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32004,
    "message": "Agent loop detected: 5 identical tool calls within 10s. Execution halted by circuit breaker."
  }
}
```

---

## ⚙️ Configuration

Propylon is configured via a single declarative `yaml` file (`configs/propylon.example.yaml`), hot-reloaded automatically upon save:

```yaml
version: "v1"

server:
  addr: "0.0.0.0:8080"
  mode: "http"

# Rate Limit & Agent Dead-Loop Protection
circuit_breaker:
  enabled: true
  max_calls_per_minute: 60
  loop_threshold: 4
  loop_window_seconds: 10

# Remote Webhook for Human-in-the-Loop Alerts
webhook:
  url: "https://your-security-webhook.internal/approvals"
  timeout_seconds: 15

policies:
  # 1. Block destructive commands
  - id: "block-dangerous-bash"
    name: "Block Destructive Shell Commands"
    target_tools: ["execute_command", "shell"]
    action: "block"
    conditions:
      param_matches:
        command:
          - '(?i)rm\s+-rf\s+/'
          - '(?i)mkfs'
    message: "Destructive command blocked by safety policy."

  # 2. Require human approval for production changes
  - id: "require-approval-for-deploy"
    name: "Production Deployment Gate"
    target_tools: ["deploy_service"]
    action: "require_approval"
    approval_channel: "terminal" # Options: terminal, webhook
    timeout: 30s
    message: "Agent is requesting a production deployment."
```

---

## 🗺️ Roadmap

- [x] **v0.1.0**: Core Rust/Axum HTTP JSON-RPC gateway & regex parameter firewall.
- [x] **v0.1.0**: Interactive terminal Human-in-the-Loop approval mechanism.
- [x] **v0.2.0**: Zero-downtime policy hot-reloading (`notify` + `arc-swap`).
- [x] **v0.2.0**: Agent dead-loop detection & rate-limiting circuit breaker.
- [x] **v0.2.0**: Remote Webhook approval channel dispatcher (Slack/Feishu/DingTalk).
- [ ] **v0.3.0**: Native `stdio` transport proxying (direct bridge for Claude Desktop).
- [ ] **v0.4.0**: Open Policy Agent (OPA / Rego) integration for complex enterprise RBAC.
- [ ] **v0.5.0**: Web Admin Console & visual trajectory inspection dashboard.

---

## 🤝 Contributing

We welcome contributions of all kinds! Whether you are implementing new MCP transports, refining security heuristics, or improving documentation:

1. Fork the repo.
2. Create a feature branch (`git checkout -b feature/amazing-feature`).
3. Commit your changes (`git commit -m 'feat: add amazing feature'`).
4. Push to the branch (`git push origin feature/amazing-feature`).
5. Open a Pull Request.

---

## 📄 License

Propylon is licensed under the [Apache 2.0 License](LICENSE).  
Copyright © 2026 xrwang.
