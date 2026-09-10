package policy

import (
	"bufio"
	"fmt"
	"os"
	"regexp"
	"strings"
	"sync"
	"time"

	"github.com/xrwang8/propylon/pkg/config"
)

// Engine evaluates tool calls against configured security policies.
type Engine struct {
	policies []*CompiledPolicy
	mu       sync.RWMutex
}

// NewEngine compiles and initializes the policy evaluation engine.
func NewEngine(configs []config.PolicyConfig) (*Engine, error) {
	compiled := make([]*CompiledPolicy, 0, len(configs))

	for _, cfg := range configs {
		cp := &CompiledPolicy{
			ID:              cfg.ID,
			Name:            cfg.Name,
			TargetTools:     make(map[string]struct{}),
			Action:          Action(cfg.Action),
			CompiledMatches: make(map[string][]*regexp.Regexp),
			Message:         cfg.Message,
		}

		for _, tool := range cfg.TargetTools {
			if tool == "*" {
				cp.MatchAllTools = true
			} else {
				cp.TargetTools[tool] = struct{}{}
			}
		}

		for param, patterns := range cfg.Conditions.ParamMatches {
			regexList := make([]*regexp.Regexp, 0, len(patterns))
			for _, pat := range patterns {
				re, err := regexp.Compile(pat)
				if err != nil {
					return nil, fmt.Errorf("invalid regex '%s' in policy '%s': %w", pat, cfg.ID, err)
				}
				regexList = append(regexList, re)
			}
			cp.CompiledMatches[param] = regexList
		}

		compiled = append(compiled, cp)
	}

	return &Engine{policies: compiled}, nil
}

// Evaluate checks an incoming tool call against all compiled policies.
func (e *Engine) Evaluate(toolName string, args map[string]interface{}) *Decision {
	e.mu.RLock()
	defer e.mu.RUnlock()

	for _, p := range e.policies {
		// 1. Tool name match
		if !p.MatchAllTools {
			if _, exists := p.TargetTools[toolName]; !exists {
				continue
			}
		}

		// 2. Check argument conditions
		matched := false
		matchedReason := p.Message

		for paramName, regexList := range p.CompiledMatches {
			val, exists := args[paramName]
			if !exists {
				continue
			}

			valStr := fmt.Sprintf("%v", val)
			for _, re := range regexList {
				if re.MatchString(valStr) {
					matched = true
					break
				}
			}
			if matched {
				break
			}
		}

		// If conditions matched or if no specific argument conditions were set
		if matched || len(p.CompiledMatches) == 0 {
			switch p.Action {
			case ActionBlock:
				return &Decision{
					Allowed:        false,
					Action:         ActionBlock,
					ViolatedPolicy: p.ID,
					Reason:         matchedReason,
				}
			case ActionRequireApproval:
				return &Decision{
					Allowed:         false,
					Action:          ActionRequireApproval,
					ViolatedPolicy:  p.ID,
					RequireApproval: true,
					ApprovalPrompt:  fmt.Sprintf("Agent requested tool '%s' with args %v. Approval required: %s", toolName, args, matchedReason),
				}
			}
		}
	}

	return &Decision{
		Allowed: true,
		Action:  ActionAllow,
	}
}

// RequestInteractiveApproval prompts the terminal operator for permission.
func RequestInteractiveApproval(prompt string, timeout time.Duration) (bool, error) {
	fmt.Printf("\n\033[1;33m[PROPYLON APPROVAL REQUIRED]\033[0m\n%s\n", prompt)
	fmt.Print("\033[1;36mDo you approve this action? (y/N): \033[0m")

	responseChan := make(chan bool, 1)
	errChan := make(chan error, 1)

	go func() {
		reader := bufio.NewReader(os.Stdin)
		input, err := reader.ReadString('\n')
		if err != nil {
			errChan <- err
			return
		}
		cleaned := strings.ToLower(strings.TrimSpace(input))
		responseChan <- (cleaned == "y" || cleaned == "yes")
	}()

	select {
	case approved := <-responseChan:
		return approved, nil
	case err := <-errChan:
		return false, err
	case <-time.After(timeout):
		fmt.Println("\n\033[1;31m[PROPYLON TIMEOUT] Approval request timed out. Action rejected by default.\033[0m")
		return false, nil
	}
}
