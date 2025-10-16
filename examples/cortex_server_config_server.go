package main

/*
#cgo LDFLAGS: -L../easytier_config_server/target/debug -leasytier_config_server
#include "../easytier_config_server/include/easytier_config_server.h"
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"fmt"
	"unsafe"
)

// ConfigServerService manages device connections and configurations
type ConfigServerService struct {
	dbURL     string
	geoipPath string
	protocol  string
	port      uint16
}

// DeviceInfo represents device information from the config server
type DeviceInfo struct {
	ClientURL *string        `json:"client_url"`
	Info      *HeartbeatInfo `json:"info"`
	Location  *Location      `json:"location"`
}

// HeartbeatInfo represents device heartbeat information
type HeartbeatInfo struct {
	MachineID               *string  `json:"machine_id"`
	UserToken               string   `json:"user_token"`
	EasyTierVersion         string   `json:"easytier_version"`
	Hostname                string   `json:"hostname"`
	RunningNetworkInstances []string `json:"running_network_instances"`
}

// Location represents geographic location
type Location struct {
	Country string  `json:"country"`
	City    *string `json:"city"`
	Region  *string `json:"region"`
}

// NetworkConfig represents network configuration
type NetworkConfig struct {
	NetworkName   string   `json:"network_name"`
	NetworkSecret string   `json:"network_secret"`
	Peers         []string `json:"peers"`
	ListenerURLs  []string `json:"listener_urls"`
	// Add other fields as needed
}

// NewConfigServerService creates a new config server service
func NewConfigServerService(dbURL, geoipPath, protocol string, port uint16) *ConfigServerService {
	return &ConfigServerService{
		dbURL:     dbURL,
		geoipPath: geoipPath,
		protocol:  protocol,
		port:      port,
	}
}

// Initialize initializes the config server singleton
func (c *ConfigServerService) Initialize() error {
	var errMsg *C.char
	defer func() {
		if errMsg != nil {
			C.free_c_char(errMsg)
		}
	}()

	cDBURL := C.CString(c.dbURL)
	defer C.free(unsafe.Pointer(cDBURL))

	var cGeoipPath *C.char
	if c.geoipPath != "" {
		cGeoipPath = C.CString(c.geoipPath)
		defer C.free(unsafe.Pointer(cGeoipPath))
	}

	success := C.create_network_config_service_singleton(
		cDBURL,
		cGeoipPath,
		&errMsg,
	)

	if !success {
		if errMsg != nil {
			return fmt.Errorf("failed to create config server: %s", C.GoString(errMsg))
		}
		return fmt.Errorf("failed to create config server")
	}

	fmt.Println("✓ Config server initialized")
	return nil
}

// Start starts the config server listener
func (c *ConfigServerService) Start() error {
	var errMsg *C.char
	defer func() {
		if errMsg != nil {
			C.free_c_char(errMsg)
		}
	}()

	cProtocol := C.CString(c.protocol)
	defer C.free(unsafe.Pointer(cProtocol))

	success := C.network_config_service_singleton_start(
		cProtocol,
		C.ushort(c.port),
		&errMsg,
	)

	if !success {
		if errMsg != nil {
			return fmt.Errorf("failed to start config server: %s", C.GoString(errMsg))
		}
		return fmt.Errorf("failed to start config server")
	}

	fmt.Printf("✓ Config server listening on %s:%d\n", c.protocol, c.port)
	return nil
}

// ListDevices lists all devices for an organization
func (c *ConfigServerService) ListDevices(orgID string) ([]DeviceInfo, error) {
	var resultJSON *C.char
	var errMsg *C.char
	defer func() {
		if resultJSON != nil {
			C.free_c_char(resultJSON)
		}
		if errMsg != nil {
			C.free_c_char(errMsg)
		}
	}()

	cOrgID := C.CString(orgID)
	defer C.free(unsafe.Pointer(cOrgID))

	success := C.network_config_service_list_devices(
		cOrgID,
		&resultJSON,
		&errMsg,
	)

	if !success {
		if errMsg != nil {
			return nil, fmt.Errorf("failed to list devices: %s", C.GoString(errMsg))
		}
		return nil, fmt.Errorf("failed to list devices")
	}

	// Parse JSON result
	var result struct {
		Devices []DeviceInfo `json:"devices"`
	}
	if err := json.Unmarshal([]byte(C.GoString(resultJSON)), &result); err != nil {
		return nil, fmt.Errorf("failed to parse devices JSON: %w", err)
	}

	return result.Devices, nil
}

// RunNetworkInstance creates and starts a network instance on a device
func (c *ConfigServerService) RunNetworkInstance(
	orgID, deviceID string,
	config NetworkConfig,
) (string, error) {
	var instIDOut *C.char
	var errMsg *C.char
	defer func() {
		if instIDOut != nil {
			C.free_c_char(instIDOut)
		}
		if errMsg != nil {
			C.free_c_char(errMsg)
		}
	}()

	cOrgID := C.CString(orgID)
	defer C.free(unsafe.Pointer(cOrgID))

	cDeviceID := C.CString(deviceID)
	defer C.free(unsafe.Pointer(cDeviceID))

	configJSON, _ := json.Marshal(config)
	cConfigJSON := C.CString(string(configJSON))
	defer C.free(unsafe.Pointer(cConfigJSON))

	success := C.network_config_service_run_network_instance(
		cOrgID,
		cDeviceID,
		cConfigJSON,
		&instIDOut,
		&errMsg,
	)

	if !success {
		if errMsg != nil {
			return "", fmt.Errorf("failed to run network: %s", C.GoString(errMsg))
		}
		return "", fmt.Errorf("failed to run network")
	}

	instID := C.GoString(instIDOut)
	fmt.Printf("✓ Network instance %s started on device %s\n", instID, deviceID)
	return instID, nil
}

// Destroy cleans up the config server singleton
func (c *ConfigServerService) Destroy() error {
	var errMsg *C.char
	defer func() {
		if errMsg != nil {
			C.free_c_char(errMsg)
		}
	}()

	success := C.destroy_network_config_service_singleton(&errMsg)
	if !success {
		if errMsg != nil {
			return fmt.Errorf("failed to destroy config server: %s", C.GoString(errMsg))
		}
		return fmt.Errorf("failed to destroy config server")
	}

	fmt.Println("✓ Config server destroyed")
	return nil
}

func main() {
	// Example usage
	dbURL := "root:password@tcp(localhost:3306)/cortex?parseTime=true&loc=UTC"
	geoipPath := "./easytier_config_server/resources/geoip2-cn.mmdb"

	configServer := NewConfigServerService(dbURL, geoipPath, "tcp", 11020)

	// Initialize
	if err := configServer.Initialize(); err != nil {
		fmt.Printf("Error initializing config server: %v\n", err)
		return
	}

	// Start listener
	if err := configServer.Start(); err != nil {
		fmt.Printf("Error starting config server: %v\n", err)
		return
	}

	fmt.Println("Config server running...")
	fmt.Println("Devices can now connect and will appear after sending heartbeat")

	// In real usage:
	// - Wait for devices to connect
	// - Admin approves devices via API
	// - Admin creates network configs
	// - Config server sends configs to devices

	// Example: List devices (after some connect)
	// devices, _ := configServer.ListDevices("org-uuid-123")
	// fmt.Printf("Connected devices: %v\n", devices)

	// Keep running
	// select {}
}
