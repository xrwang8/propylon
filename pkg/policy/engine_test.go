package policy

import (
	"testing"

	"github.com/xrwang8/propylon/pkg/config"
)

func TestPolicyEngineBlock(t *testing.T) {
	configs := []config.PolicyConfig{
		{
			ID:          "block-dangerous-bash",
			Name:        "Block Destructive Commands",
			TargetTools: []string{"execute_command"},
			Action:      "block",
			Conditions: config.PolicyConditions{
				ParamMatches: map[string][]string{
					"command": {`(?i)rm\s+-rf\s+/`},
				},
			},
			Message: "Destructive command blocked.",
		},
	}

	engine, err := NewEngine(configs)
	if err != nil {
		t.Fatalf("Failed to create engine: %v", err)
	}

	// Test 1: Dangerous command should be blocked
	decision := engine.Evaluate("execute_command", map[string]interface{}{
		"command": "rm -rf / --no-preserve-root",
	})
	if decision.Allowed {
		t.Errorf("Expected command to be blocked, but was allowed")
	}
	if decision.ViolatedPolicy != "block-dangerous-bash" {
		t.Errorf("Expected policy 'block-dangerous-bash', got '%s'", decision.ViolatedPolicy)
	}

	// Test 2: Safe command should be allowed
	decisionSafe := engine.Evaluate("execute_command", map[string]interface{}{
		"command": "ls -la /var/log",
	})
	if !decisionSafe.Allowed {
		t.Errorf("Expected safe command to be allowed, but was blocked: %s", decisionSafe.Reason)
	}
}
