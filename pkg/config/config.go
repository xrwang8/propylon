package config

import (
	"fmt"
	"os"
	"time"

	"gopkg.in/yaml.v3"
)

// Config represents the top-level configuration for Propylon Gateway.
type Config struct {
	Version   string          `yaml:"version"`
	Server    ServerConfig    `yaml:"server"`
	Audit     AuditConfig     `yaml:"audit"`
	Upstreams []UpstreamConfig `yaml:"upstreams"`
	Policies  []PolicyConfig  `yaml:"policies"`
}

// ServerConfig defines the gateway listener settings.
type ServerConfig struct {
	Addr         string        `yaml:"addr"`
	Mode         string        `yaml:"mode"` // http, sse, stdio
	ReadTimeout  time.Duration `yaml:"read_timeout"`
	WriteTimeout time.Duration `yaml:"write_timeout"`
}

// AuditConfig defines telemetry and audit logging behavior.
type AuditConfig struct {
	Enabled           bool   `yaml:"enabled"`
	Output            string `yaml:"output"` // stdout, file
	LogArguments      bool   `yaml:"log_arguments"`
	MaskSensitiveData bool   `yaml:"mask_sensitive_data"`
}

// UpstreamConfig defines a backend MCP server or tool provider.
type UpstreamConfig struct {
	Name        string   `yaml:"name"`
	Type        string   `yaml:"type"` // mcp-sse, mcp-stdio, http
	Target      string   `yaml:"target,omitempty"`
	Command     string   `yaml:"command,omitempty"`
	Args        []string `yaml:"args,omitempty"`
	Description string   `yaml:"description,omitempty"`
}

// PolicyConfig defines security rules, actions, and criteria.
type PolicyConfig struct {
	ID              string             `yaml:"id"`
	Name            string             `yaml:"name"`
	TargetTools     []string           `yaml:"target_tools"`
	Action          string             `yaml:"action"` // block, allow, require_approval, mask
	Conditions      PolicyConditions   `yaml:"conditions"`
	ApprovalChannel string             `yaml:"approval_channel,omitempty"` // terminal, webhook
	Timeout         time.Duration      `yaml:"timeout,omitempty"`
	Message         string             `yaml:"message"`
	Masks           []MaskRule         `yaml:"masks,omitempty"`
}

// PolicyConditions contains matching criteria for tool arguments.
type PolicyConditions struct {
	ParamMatches map[string][]string `yaml:"param_matches"` // argument_name -> list of regexes
}

// MaskRule defines a pattern to search for and replace in outputs.
type MaskRule struct {
	Pattern     string `yaml:"pattern"`
	Replacement string `yaml:"replacement"`
}

// Load reads and parses a YAML configuration file.
func Load(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file %s: %w", path, err)
	}

	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("failed to parse yaml config: %w", err)
	}

	if cfg.Server.Addr == "" {
		cfg.Server.Addr = "127.0.0.1:8080"
	}
	if cfg.Server.Mode == "" {
		cfg.Server.Mode = "http"
	}

	return &cfg, nil
}
