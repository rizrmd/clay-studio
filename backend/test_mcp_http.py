#!/usr/bin/env python3
"""Simple test script to verify MCP HTTP transport"""

import requests
import json

def test_mcp_http_server(port=8001):
    """Test MCP server HTTP transport"""
    base_url = f"http://localhost:{port}"
    
    # Test basic connectivity
    try:
        response = requests.get(f"{base_url}/mcp/sse", timeout=5)
        print(f"SSE endpoint status: {response.status_code}")
    except Exception as e:
        print(f"SSE endpoint failed: {e}")
    
    # Test MCP initialize request
    initialize_request = {
        "jsonrpc": "2.0",
        "id": 0,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {"roots": {}},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    }
    
    try:
        response = requests.post(
            f"{base_url}/mcp",
            json=initialize_request,
            headers={"Content-Type": "application/json"},
            timeout=5
        )
        print(f"Initialize request status: {response.status_code}")
        if response.status_code == 200:
            print(f"Response: {response.json()}")
        else:
            print(f"Error response: {response.text}")
    except Exception as e:
        print(f"Initialize request failed: {e}")

if __name__ == "__main__":
    print("Testing MCP HTTP transport...")
    test_mcp_http_server()