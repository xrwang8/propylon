package policy

import "regexp"

// Action represents the outcome of a policy evaluation.
type Action string

const (
	ActionAllow           Action = "allow"
	ActionBlock           Action = "block"
	ActionRequireApproval Action = "require_approval"
	ActionMask            Action = "mask"
)

// Decision represents the final result of evaluating a request against policies.
type Decision struct {
	Allowed         bool
	Action          Action
	ViolatedPolicy  string
	Reason          string
	RequireApproval bool
	ApprovalPrompt  string
}

// CompiledPolicy holds precompiled regular expressions for runtime performance.
type CompiledPolicy struct {
	ID              string
	Name            string
	TargetTools     map[string]struct{}
	MatchAllTools   bool
	Action          Action
	CompiledMatches map[string][]*regexp.Regexp
	Message         string
}
