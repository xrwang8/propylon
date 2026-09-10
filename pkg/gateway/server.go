package gateway

import (
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"time"

	"github.com/xrwang8/propylon/pkg/config"
	"github.com/xrwang8/propylon/pkg/policy"
	"github.com/xrwang8/propylon/pkg/protocol/mcp"
)

// Server handles incoming MCP and tool execution traffic.
type Server struct {
	cfg          *config.Config
	policyEngine *policy.Engine
	httpServer   *http.Server
}

// NewServer creates a new Propylon gateway server instance.
func NewServer(cfg *config.Config, engine *policy.Engine) *Server {
	s := &Server{
		cfg:          cfg,
		policyEngine: engine,
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/healthz", s.handleHealthz)
	mux.HandleFunc("/v1/mcp", s.handleMCP)
	mux.HandleFunc("/", s.handleMCP) // Default JSON-RPC endpoint

	s.httpServer = &http.Server{
		Addr:         cfg.Server.Addr,
		Handler:      mux,
		ReadTimeout:  cfg.Server.ReadTimeout,
		WriteTimeout: cfg.Server.WriteTimeout,
	}

	return s
}

// Start begins listening for requests.
func (s *Server) Start() error {
	log.Printf("[Propylon] Gateway listening on http://%s (mode: %s)", s.cfg.Server.Addr, s.cfg.Server.Mode)
	return s.httpServer.ListenAndServe()
}

// Close cleanly shuts down the server.
func (s *Server) Close() error {
	return s.httpServer.Close()
}

func (s *Server) handleHealthz(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	_ = json.NewEncoder(w).Encode(map[string]string{
		"status":  "healthy",
		"gateway": "Propylon",
		"version": s.cfg.Version,
	})
}

// handleMCP intercepts, inspects, and forwards JSON-RPC / MCP requests.
func (s *Server) handleMCP(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method Not Allowed. Use POST for JSON-RPC MCP calls.", http.StatusMethodNotAllowed)
		return
	}

	body, err := io.ReadAll(r.Body)
	if err != nil {
		s.writeJSONRPCError(w, nil, mcp.CodeParseError, "Failed to read request body")
		return
	}
	defer r.Body.Close()

	var req mcp.Request
	if err := json.Unmarshal(body, &req); err != nil {
		s.writeJSONRPCError(w, nil, mcp.CodeParseError, "Invalid JSON-RPC payload")
		return
	}

	// Route inspection based on method
	switch req.Method {
	case "tools/call":
		s.handleToolsCall(w, &req)
	default:
		// Default pass-through or mock response for discovery methods (tools/list, etc.)
		s.handlePassThrough(w, &req)
	}
}

func (s *Server) handleToolsCall(w http.ResponseWriter, req *mcp.Request) {
	var callParams mcp.CallToolParams
	if err := json.Unmarshal(req.Params, &callParams); err != nil {
		s.writeJSONRPCError(w, req.ID, mcp.CodeInvalidParams, "Invalid tool call arguments")
		return
	}

	// 1. Audit Logging
	if s.cfg.Audit.Enabled {
		log.Printf("[Propylon Audit] Inspecting Tool Call: '%s' | Args: %v", callParams.Name, callParams.Arguments)
	}

	// 2. Policy Evaluation
	decision := s.policyEngine.Evaluate(callParams.Name, callParams.Arguments)

	// 3. Handle Policy Action
	if !decision.Allowed {
		if decision.RequireApproval {
			// Human-in-the-Loop flow
			approved, err := policy.RequestInteractiveApproval(decision.ApprovalPrompt, 30*time.Second)
			if err != nil || !approved {
				log.Printf("\033[1;31m[Propylon Policy] REJECTED tool '%s' by operator\033[0m", callParams.Name)
				s.writeJSONRPCError(w, req.ID, mcp.CodeApprovalDenied, "Action rejected by security supervisor")
				return
			}
			log.Printf("\033[1;32m[Propylon Policy] APPROVED tool '%s' by operator\033[0m", callParams.Name)
		} else {
			// Direct Block
			log.Printf("\033[1;31m[Propylon Policy] BLOCKED tool '%s': %s (Policy: %s)\033[0m",
				callParams.Name, decision.Reason, decision.ViolatedPolicy)
			s.writeJSONRPCError(w, req.ID, mcp.CodePolicyViolation, fmt.Sprintf("Blocked by Propylon policy '%s': %s", decision.ViolatedPolicy, decision.Reason))
			return
		}
	}

	// 4. If allowed, simulate successful execution or proxy to upstream
	log.Printf("\033[1;32m[Propylon Policy] PASSED tool call '%s'\033[0m", callParams.Name)
	resp := mcp.Response{
		JSONRPC: mcp.JSONRPCVersion,
		ID:      req.ID,
		Result: mcp.CallToolResult{
			Content: []mcp.ToolContent{
				{
					Type: "text",
					Text: fmt.Sprintf("[Propylon Verified] Successfully forwarded and executed tool '%s'", callParams.Name),
				},
			},
		},
	}
	s.writeJSONResponse(w, resp)
}

func (s *Server) handlePassThrough(w http.ResponseWriter, req *mcp.Request) {
	// Standard response for demo / discovery
	resp := mcp.Response{
		JSONRPC: mcp.JSONRPCVersion,
		ID:      req.ID,
		Result: map[string]interface{}{
			"message": "Propylon Gateway pass-through",
			"method":  req.Method,
		},
	}
	s.writeJSONResponse(w, resp)
}

func (s *Server) writeJSONRPCError(w http.ResponseWriter, id interface{}, code int, message string) {
	resp := mcp.Response{
		JSONRPC: mcp.JSONRPCVersion,
		ID:      id,
		Error: &mcp.Error{
			Code:    code,
			Message: message,
		},
	}
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK) // JSON-RPC 2.0 returns 200 with error object
	_ = json.NewEncoder(w).Encode(resp)
}

func (s *Server) writeJSONResponse(w http.ResponseWriter, resp mcp.Response) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	_ = json.NewEncoder(w).Encode(resp)
}
