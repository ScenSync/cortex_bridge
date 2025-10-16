package main

/*
#cgo LDFLAGS: -L../easytier_network_gateway/target/debug -leasytier_network_gateway
#include "../easytier_network_gateway/include/easytier_network_gateway.h"
#include <stdlib.h>
*/
import "C"
import (
	"fmt"
	"unsafe"
)

// GatewayConfig represents the configuration for the EasyTier gateway
type GatewayConfig struct {
	InstanceName            string
	NetworkName             string
	NetworkSecret           string
	DHCP                    bool
	IPv4                    string
	IPv6                    string
	ListenerURLs            []string
	PeerURLs                []string
	RPCPort                 int
	DefaultProtocol         string
	DevName                 string
	EnableEncryption        bool
	EnableIPv6              bool
	MTU                     int
	LatencyFirst            bool
	EnableExitNode          bool
	NoTun                   bool
	UseSmoltcp              bool
	ForeignNetworkWhitelist string
	DisableP2P              bool
	RelayAllPeerRPC         bool
	DisableUDPHolePunching  bool
	PrivateMode             bool
}

// GatewayService manages the EasyTier gateway instance
type GatewayService struct {
	instanceName string
	config       *GatewayConfig
}

// NewGatewayService creates a new gateway service
func NewGatewayService(config *GatewayConfig) *GatewayService {
	return &GatewayService{
		instanceName: config.InstanceName,
		config:       config,
	}
}

// Start starts the gateway instance
func (g *GatewayService) Start() error {
	// Convert strings to C strings
	cInstanceName := C.CString(g.config.InstanceName)
	defer C.free(unsafe.Pointer(cInstanceName))

	cNetworkName := C.CString(g.config.NetworkName)
	defer C.free(unsafe.Pointer(cNetworkName))

	cNetworkSecret := C.CString(g.config.NetworkSecret)
	defer C.free(unsafe.Pointer(cNetworkSecret))

	cIPv4 := C.CString(g.config.IPv4)
	defer C.free(unsafe.Pointer(cIPv4))

	cIPv6 := C.CString(g.config.IPv6)
	defer C.free(unsafe.Pointer(cIPv6))

	cDefaultProtocol := C.CString(g.config.DefaultProtocol)
	defer C.free(unsafe.Pointer(cDefaultProtocol))

	cDevName := C.CString(g.config.DevName)
	defer C.free(unsafe.Pointer(cDevName))

	cForeignWhitelist := C.CString(g.config.ForeignNetworkWhitelist)
	defer C.free(unsafe.Pointer(cForeignWhitelist))

	// Convert listener URLs
	cListeners := make([]*C.char, len(g.config.ListenerURLs))
	for i, url := range g.config.ListenerURLs {
		cListeners[i] = C.CString(url)
		defer C.free(unsafe.Pointer(cListeners[i]))
	}

	// Convert peer URLs
	cPeers := make([]*C.char, len(g.config.PeerURLs))
	for i, url := range g.config.PeerURLs {
		cPeers[i] = C.CString(url)
		defer C.free(unsafe.Pointer(cPeers[i]))
	}

	// Build C config struct
	var cConfig C.EasyTierCoreConfig
	cConfig.instance_name = cInstanceName
	cConfig.network_name = cNetworkName
	cConfig.network_secret = cNetworkSecret
	cConfig.dhcp = boolToInt(g.config.DHCP)
	cConfig.ipv4 = cIPv4
	cConfig.ipv6 = cIPv6
	cConfig.rpc_port = C.int(g.config.RPCPort)
	cConfig.default_protocol = cDefaultProtocol
	cConfig.dev_name = cDevName
	cConfig.enable_encryption = boolToInt(g.config.EnableEncryption)
	cConfig.enable_ipv6 = boolToInt(g.config.EnableIPv6)
	cConfig.mtu = C.int(g.config.MTU)
	cConfig.latency_first = boolToInt(g.config.LatencyFirst)
	cConfig.enable_exit_node = boolToInt(g.config.EnableExitNode)
	cConfig.no_tun = boolToInt(g.config.NoTun)
	cConfig.use_smoltcp = boolToInt(g.config.UseSmoltcp)
	cConfig.foreign_network_whitelist = cForeignWhitelist
	cConfig.disable_p2p = boolToInt(g.config.DisableP2P)
	cConfig.relay_all_peer_rpc = boolToInt(g.config.RelayAllPeerRPC)
	cConfig.disable_udp_hole_punching = boolToInt(g.config.DisableUDPHolePunching)
	cConfig.private_mode = boolToInt(g.config.PrivateMode)

	if len(cListeners) > 0 {
		cConfig.listener_urls = &cListeners[0]
		cConfig.listener_urls_count = C.int(len(cListeners))
	}

	if len(cPeers) > 0 {
		cConfig.peer_urls = &cPeers[0]
		cConfig.peer_urls_count = C.int(len(cPeers))
	}

	// Call FFI
	result := C.start_easytier_core(&cConfig)
	if result != 0 {
		errMsg := C.easytier_common_get_error_msg()
		if errMsg != nil {
			return fmt.Errorf("failed to start gateway: %s", C.GoString(errMsg))
		}
		return fmt.Errorf("failed to start gateway (unknown error)")
	}

	fmt.Printf("✓ Gateway '%s' started successfully\n", g.config.InstanceName)
	return nil
}

// Stop stops the gateway instance
func (g *GatewayService) Stop() error {
	cName := C.CString(g.instanceName)
	defer C.free(unsafe.Pointer(cName))

	result := C.stop_easytier_core(cName)
	if result != 0 {
		return fmt.Errorf("failed to stop gateway")
	}

	fmt.Printf("✓ Gateway '%s' stopped\n", g.instanceName)
	return nil
}

func boolToInt(b bool) C.int {
	if b {
		return 1
	}
	return 0
}

func main() {
	// Example gateway configuration
	config := &GatewayConfig{
		InstanceName:  "cortex-server-gateway",
		NetworkName:   "cortex-vpn",
		NetworkSecret: "your-secret-key-here",
		DHCP:          false,
		IPv4:          "10.144.144.1", // Server's VPN IP
		ListenerURLs: []string{
			"tcp://0.0.0.0:11010",
			"udp://0.0.0.0:11011",
			"ws://0.0.0.0:11012",
		},
		PeerURLs:                []string{}, // Server is the gateway
		RPCPort:                 15888,
		DefaultProtocol:         "tcp",
		DevName:                 "",
		EnableEncryption:        true,
		EnableIPv6:              true,
		MTU:                     1380,
		LatencyFirst:            false,
		EnableExitNode:          false,
		NoTun:                   false,
		UseSmoltcp:              false,
		ForeignNetworkWhitelist: "*",
		DisableP2P:              false,
		RelayAllPeerRPC:         false,
		DisableUDPHolePunching:  false,
		PrivateMode:             true, // Server creates the network
	}

	gateway := NewGatewayService(config)

	if err := gateway.Start(); err != nil {
		fmt.Printf("Error: %v\n", err)
		return
	}

	fmt.Println("Gateway running... Press Ctrl+C to stop")

	// In real usage, wait for signal
	// select {}
}
