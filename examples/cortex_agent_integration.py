#!/usr/bin/env python3
"""
Example Python integration code for cortex_agent using easytier_device_client

This example shows how to use the new easytier_device_client library
to connect a device to the cortex_server config server.
"""

from ctypes import CDLL, c_char_p, c_int, c_void_p, POINTER, Structure, pointer
import uuid
import os

class CortexWebClient(Structure):
    """C struct for web client configuration"""
    _fields_ = [
        ("config_server_url", c_char_p),
        ("organization_id", c_char_p),
        ("machine_id", c_char_p),
    ]

class CortexNetworkInfo(Structure):
    """C struct for network information"""
    _fields_ = [
        ("instance_name", c_char_p),
        ("network_name", c_char_p),
        ("virtual_ipv4", c_char_p),
        ("hostname", c_char_p),
        ("version", c_char_p),
    ]

class EasyTierDeviceClient:
    """Python wrapper for easytier_device_client library"""
    
    def __init__(self, lib_path: str = "./network/lib/libeasytier_device_client.so"):
        """
        Initialize the EasyTier device client library
        
        Args:
            lib_path: Path to libeasytier_device_client shared library
        """
        self.lib = CDLL(lib_path)
        
        # Define function signatures
        self.lib.cortex_start_web_client.argtypes = [POINTER(CortexWebClient)]
        self.lib.cortex_start_web_client.restype = c_int
        
        self.lib.cortex_stop_web_client.argtypes = [c_char_p]
        self.lib.cortex_stop_web_client.restype = c_int
        
        self.lib.cortex_get_web_client_network_info.argtypes = [c_char_p, POINTER(POINTER(CortexNetworkInfo))]
        self.lib.cortex_get_web_client_network_info.restype = c_int
        
        self.lib.easytier_common_get_error_msg.argtypes = []
        self.lib.easytier_common_get_error_msg.restype = c_char_p
        
        self.instance_name = None
    
    def start(self, config_server_url: str, machine_id: str = None) -> bool:
        """
        Start web client connection to config server
        
        Args:
            config_server_url: URL of the config server with organization ID in path (e.g., "tcp://server.com:11020/org-123")
            machine_id: Persistent device UUID (optional, auto-generated if None)
        
        Returns:
            True if successful, False otherwise
        """
        if machine_id is None:
            machine_id = str(uuid.uuid4())
        
        config = CortexWebClient(
            config_server_url=config_server_url.encode(),
            machine_id=machine_id.encode()
        )
        
        result = self.lib.cortex_start_web_client(pointer(config))
        
        if result == 0:
            # Extract organization ID from URL path for instance name
            from urllib.parse import urlparse
            parsed = urlparse(config_server_url)
            self.instance_name = parsed.path.strip('/')
            print(f"✓ Web client started successfully")
            print(f"  Organization ID: {self.instance_name}")
            print(f"  Machine ID: {machine_id}")
            print(f"  Config Server: {config_server_url}")
            return True
        else:
            error_msg = self.lib.easytier_common_get_error_msg()
            if error_msg:
                print(f"✗ Failed to start web client: {error_msg.decode()}")
            return False
    
    def stop(self) -> bool:
        """Stop the web client"""
        if not self.instance_name:
            print("No active instance to stop")
            return False
        
        result = self.lib.cortex_stop_web_client(self.instance_name.encode())
        if result == 0:
            print(f"✓ Web client stopped")
            self.instance_name = None
            return True
        else:
            print(f"✗ Failed to stop web client")
            return False
    
    def get_network_info(self) -> dict:
        """Get network information for the running instance"""
        if not self.instance_name:
            return None
        
        info_ptr = POINTER(CortexNetworkInfo)()
        result = self.lib.cortex_get_web_client_network_info(
            self.instance_name.encode(),
            pointer(info_ptr)
        )
        
        if result == 0 and info_ptr:
            info = info_ptr.contents
            return {
                "instance_name": info.instance_name.decode() if info.instance_name else "",
                "network_name": info.network_name.decode() if info.network_name else "",
                "virtual_ipv4": info.virtual_ipv4.decode() if info.virtual_ipv4 else "",
                "hostname": info.hostname.decode() if info.hostname else "",
                "version": info.version.decode() if info.version else "",
            }
        
        return None

# Example usage
def main():
    # Configuration
    CONFIG_SERVER_URL = "tcp://192.168.1.100:11020"  # Your cortex_server config server
    ORGANIZATION_ID = "550e8400-e29b-41d4-a716-446655440000"  # From cortex_server
    
    # Optionally load persistent machine_id
    MACHINE_ID_FILE = "/var/lib/cortex_agent/machine_id"
    if os.path.exists(MACHINE_ID_FILE):
        with open(MACHINE_ID_FILE, 'r') as f:
            machine_id = f.read().strip()
    else:
        machine_id = str(uuid.uuid4())
        os.makedirs(os.path.dirname(MACHINE_ID_FILE), exist_ok=True)
        with open(MACHINE_ID_FILE, 'w') as f:
            f.write(machine_id)
    
    # Create client
    client = EasyTierDeviceClient()
    
    # Start connection (organization ID is embedded in URL path)
    if client.start(f"{CONFIG_SERVER_URL}/{ORGANIZATION_ID}", machine_id):
        print("Web client connected successfully")
        print("Waiting for admin approval and network configuration...")
        print("Device will automatically join VPN networks when configured by admin")
        
        # In a real application, you'd keep the client running
        # and handle network lifecycle events
        
        # For demo purposes, get status and stop
        import time
        time.sleep(5)
        
        info = client.get_network_info()
        if info:
            print(f"Network Info: {info}")
        
        # Stop (in real usage, keep running)
        # client.stop()
    else:
        print("Failed to start web client")
        return 1
    
    return 0

if __name__ == "__main__":
    import sys
    sys.exit(main())

