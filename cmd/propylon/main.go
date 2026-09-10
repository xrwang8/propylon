package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/xrwang8/propylon/pkg/config"
	"github.com/xrwang8/propylon/pkg/gateway"
	"github.com/xrwang8/propylon/pkg/policy"
)

const version = "0.1.0-alpha"

const banner = `
  ____                               _               
 |  _ \ _ __ ___  _ __  _   _| | ___  _ __  
 | |_) | '__/ _ \| '_ \| | | | |/ _ \| '_ \ 
 |  __/| | | (_) | |_) | |_| | | (_) | | | |
 |_|   |_|  \___/| .__/ \__, |_|\___/|_| |_|
                 |_|    |___/               
    The Core Security Gateway & Firewall for AI Agents
    Version: %s | Greek Origin: Προπύλαιον
------------------------------------------------------------
`

func main() {
	configPath := flag.String("config", "configs/propylon.example.yaml", "Path to Propylon configuration file")
	showVersion := flag.Bool("version", false, "Print version information and exit")
	flag.Parse()

	if *showVersion {
		fmt.Printf("Propylon Gateway v%s\n", version)
		os.Exit(0)
	}

	fmt.Printf(banner, version)

	// 1. Load configuration
	log.Printf("[Init] Loading configuration from: %s", *configPath)
	cfg, err := config.Load(*configPath)
	if err != nil {
		log.Fatalf("[Error] Failed to load config: %v", err)
	}

	// 2. Initialize Policy Engine
	log.Printf("[Init] Compiling %d security policies...", len(cfg.Policies))
	engine, err := policy.NewEngine(cfg.Policies)
	if err != nil {
		log.Fatalf("[Error] Failed to initialize policy engine: %v", err)
	}

	// 3. Initialize Gateway Server
	server := gateway.NewServer(cfg, engine)

	// 4. Handle OS signals for graceful shutdown
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, os.Interrupt, syscall.SIGTERM)

	go func() {
		if err := server.Start(); err != nil {
			log.Printf("[Server] Shutting down: %v", err)
		}
	}()

	log.Printf("[Ready] Propylon Gateway is actively guarding AI tool invocations.")

	<-stop
	log.Println("[Shutdown] Terminating Propylon Gateway gracefully...")
	_ = server.Close()
	log.Println("[Shutdown] Goodbye.")
}
