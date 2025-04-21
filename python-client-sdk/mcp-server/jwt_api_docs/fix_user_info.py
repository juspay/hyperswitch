#!/usr/bin/env python3
"""
Fix for User Info API Endpoint

This script tests different API endpoints for the user info API
to find the correct one that works with the JWT token.

Usage:
    python fix_user_info.py --token YOUR_JWT_TOKEN
"""

import argparse
import json
import requests
import logging
import sys
from typing import Dict, Any

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    handlers=[
        logging.FileHandler("fix_user_info.log"),
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)

# Potential API base URLs to try
API_BASES = [
    "http://localhost:8080",
]

# Potential endpoint paths to try
ENDPOINT_PATHS = [
    "/user/info",
    "/v1/user/info",
    "/v1/users/me",
    "/me",
    "/user",
    "/user/me",
    "/v1/account",
]

def test_endpoint(base_url: str, endpoint: str, jwt_token: str) -> Dict[str, Any]:
    """Test a specific API endpoint with the JWT token"""
    url = f"{base_url}{endpoint}"
    headers = {
        "Authorization": f"Bearer {jwt_token}",
        "Accept": "application/json"
    }
    
    try:
        logger.info(f"Testing endpoint: {url}")
        response = requests.get(url, headers=headers, timeout=5)
        
        # Extract relevant information about the response
        status_code = response.status_code
        content_type = response.headers.get('Content-Type', '')
        
        try:
            # Try to parse JSON response
            response_json = response.json()
            logger.info(f"Response [{status_code}]: {json.dumps(response_json, indent=2)}")
            return {
                "url": url,
                "status_code": status_code,
                "response": response_json,
                "success": 200 <= status_code < 300
            }
        except json.JSONDecodeError:
            # Not a JSON response
            logger.info(f"Response [{status_code}] (not JSON): {response.text[:100]}...")
            return {
                "url": url,
                "status_code": status_code,
                "response": response.text[:200],
                "success": False
            }
            
    except requests.RequestException as e:
        logger.error(f"Error testing {url}: {str(e)}")
        return {
            "url": url,
            "status_code": None,
            "error": str(e),
            "success": False
        }

def find_working_endpoint(jwt_token: str) -> Dict[str, Any]:
    """Try different endpoints to find one that works with the JWT token"""
    results = []
    working_endpoints = []
    
    # Try all combinations of base URLs and endpoints
    for base in API_BASES:
        for path in ENDPOINT_PATHS:
            result = test_endpoint(base, path, jwt_token)
            results.append(result)
            
            if result.get("success", False):
                working_endpoints.append(result)
    
    # Return summary of results
    return {
        "all_results": results,
        "working_endpoints": working_endpoints,
        "success": len(working_endpoints) > 0
    }

def main():
    """Main function"""
    parser = argparse.ArgumentParser(description="Fix for User Info API endpoint")
    parser.add_argument("--token", required=True, help="JWT token for authentication")
    args = parser.parse_args()
    
    jwt_token = args.token
    
    logger.info("Starting to find the correct User Info API endpoint")
    results = find_working_endpoint(jwt_token)
    
    print("\n" + "=" * 60)
    print("RESULTS SUMMARY")
    print("=" * 60)
    
    if results["success"]:
        print(f"\n✅ Found {len(results['working_endpoints'])} working endpoints!")
        for i, endpoint in enumerate(results['working_endpoints'], 1):
            print(f"\n{i}. Working endpoint: {endpoint['url']}")
            print(f"   Status code: {endpoint['status_code']}")
            print(f"   Response: {json.dumps(endpoint['response'], indent=2)}")
        
        # Recommend a solution
        recommended = results['working_endpoints'][0]
        url_parts = recommended['url'].split('/')
        base_url = '/'.join(url_parts[:3])  # http://example.com
        endpoint = '/' + '/'.join(url_parts[3:])  # /path/to/endpoint
        
        print("\nRecommended fix:")
        print(f"1. Edit the file: hyperswitch_mcp/user.py")
        print(f"2. Change the hardcoded URL from 'http://localhost:8080/user/info' to '{recommended['url']}'")
        print(f"   Or set up Configuration with:")
        print(f"   - host = '{base_url}'")
        print(f"   - endpoint = '{endpoint}'")
    else:
        print("\n❌ No working endpoints found.")
        print("All tested endpoints returned errors or non-successful status codes.")
        print("\nPossible next steps:")
        print("1. Verify that the JWT token is valid and not expired")
        print("2. Check the API documentation for the correct endpoint")
        print("3. Contact the API provider for support")
    
    print("\n" + "=" * 60)
    return 0 if results["success"] else 1

if __name__ == "__main__":
    sys.exit(main()) 