#!/usr/bin/env python3
"""
Business Profiles API Test Script

This script tests the Business Profiles API endpoints 
using only localhost:8080 as the base URL.

Usage:
    python test_business_profiles.py --token YOUR_JWT_TOKEN --api_key YOUR_API_KEY
"""

import argparse
import json
import requests
import logging
import sys
import time
from typing import Dict, Any, List, Optional
from urllib.parse import urljoin

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    handlers=[
        logging.FileHandler("business_profiles_test.log"),
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)

# Base URL - strictly localhost:8080 only
BASE_URL = "http://localhost:8080"

class BusinessProfilesTest:
    """Test class for Business Profiles API endpoints."""
    
    def __init__(self, jwt_token: str, api_key: str, account_id: str):
        """Initialize with authentication credentials."""
        self.jwt_token = jwt_token
        self.api_key = api_key
        self.account_id = account_id
        self.headers = {
            "Authorization": f"Bearer {jwt_token}",
            "api-key": api_key,
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
        
        # Store test results
        self.test_results = []
    
    def _make_request(self, method: str, endpoint: str, data: Dict = None) -> Dict[str, Any]:
        """Make an API request and return the response."""
        url = urljoin(BASE_URL, endpoint)
        logger.info(f"Making {method} request to {url}")
        
        try:
            if method == "GET":
                response = requests.get(url, headers=self.headers, timeout=10)
            elif method == "POST":
                response = requests.post(url, headers=self.headers, json=data, timeout=10)
            elif method == "PUT":
                response = requests.put(url, headers=self.headers, json=data, timeout=10)
            elif method == "DELETE":
                response = requests.delete(url, headers=self.headers, timeout=10)
            else:
                return {"error": f"Unsupported method: {method}"}
            
            # Log response details
            status_code = response.status_code
            content_type = response.headers.get('Content-Type', '')
            
            try:
                # Try to parse as JSON
                response_json = response.json()
                logger.info(f"Response [{status_code}]: {json.dumps(response_json, indent=2)}")
                return {
                    "status_code": status_code,
                    "response": response_json,
                    "success": 200 <= status_code < 300
                }
            except json.JSONDecodeError:
                # Not a JSON response
                logger.info(f"Response [{status_code}] (not JSON): {response.text[:100]}...")
                return {
                    "status_code": status_code,
                    "response": response.text,
                    "success": False
                }
                
        except requests.RequestException as e:
            logger.error(f"Error making request: {str(e)}")
            return {
                "error": str(e),
                "success": False
            }
    
    def test_list_profiles(self) -> Dict[str, Any]:
        """Test listing all business profiles."""
        test_name = "List Business Profiles"
        logger.info(f"Running test: {test_name}")
        
        endpoint = f"/account/{self.account_id}/business_profile"
        result = self._make_request("GET", endpoint)
        
        # Record test result
        success = result.get("success", False)
        self.test_results.append({
            "test_name": test_name,
            "success": success,
            "status_code": result.get("status_code"),
            "message": "Successfully retrieved profiles" if success else "Failed to retrieve profiles"
        })
        
        return result
    
    def test_get_profile(self, profile_id: str) -> Dict[str, Any]:
        """Test retrieving a specific business profile."""
        test_name = f"Get Business Profile {profile_id}"
        logger.info(f"Running test: {test_name}")
        
        endpoint = f"/account/{self.account_id}/business_profile/{profile_id}"
        result = self._make_request("GET", endpoint)
        
        # Record test result
        success = result.get("success", False)
        self.test_results.append({
            "test_name": test_name,
            "success": success,
            "status_code": result.get("status_code"),
            "message": f"Successfully retrieved profile {profile_id}" if success else f"Failed to retrieve profile {profile_id}"
        })
        
        return result
    
    def test_create_profile(self, profile_data: Dict[str, Any]) -> Dict[str, Any]:
        """Test creating a new business profile."""
        test_name = "Create Business Profile"
        logger.info(f"Running test: {test_name}")
        
        endpoint = f"/account/{self.account_id}/business_profile"
        result = self._make_request("POST", endpoint, profile_data)
        
        # Record test result
        success = result.get("success", False)
        self.test_results.append({
            "test_name": test_name,
            "success": success,
            "status_code": result.get("status_code"),
            "message": "Successfully created profile" if success else "Failed to create profile"
        })
        
        return result
    
    def test_update_profile(self, profile_id: str, profile_data: Dict[str, Any]) -> Dict[str, Any]:
        """Test updating an existing business profile."""
        test_name = f"Update Business Profile {profile_id}"
        logger.info(f"Running test: {test_name}")
        
        endpoint = f"/account/{self.account_id}/business_profile/{profile_id}"
        result = self._make_request("PUT", endpoint, profile_data)
        
        # Record test result
        success = result.get("success", False)
        self.test_results.append({
            "test_name": test_name,
            "success": success,
            "status_code": result.get("status_code"),
            "message": f"Successfully updated profile {profile_id}" if success else f"Failed to update profile {profile_id}"
        })
        
        return result
    
    def test_delete_profile(self, profile_id: str) -> Dict[str, Any]:
        """Test deleting a business profile."""
        test_name = f"Delete Business Profile {profile_id}"
        logger.info(f"Running test: {test_name}")
        
        endpoint = f"/account/{self.account_id}/business_profile/{profile_id}"
        result = self._make_request("DELETE", endpoint)
        
        # Record test result
        success = result.get("success", False)
        self.test_results.append({
            "test_name": test_name,
            "success": success,
            "status_code": result.get("status_code"),
            "message": f"Successfully deleted profile {profile_id}" if success else f"Failed to delete profile {profile_id}"
        })
        
        return result
    
    def run_all_tests(self) -> None:
        """Run all business profile tests."""
        logger.info("Starting Business Profiles API tests")
        
        # Step 1: List profiles
        list_result = self.test_list_profiles()
        
        # Step 2: Create a new profile
        new_profile_data = {
            "profile_name": f"Test Profile {int(time.time())}",
            "description": "Profile created by automated test",
            "return_url": "http://example.com/return",
            "webhook_url": "http://example.com/webhook",
            "webhook_version": "1.0.0",
            "metadata": {
                "test_key": "test_value",
                "created_by": "automated_test"
            }
        }
        
        create_result = self.test_create_profile(new_profile_data)
        
        # If profile creation succeeded, continue with other tests
        if create_result.get("success", False):
            profile_id = create_result.get("response", {}).get("profile_id")
            
            if profile_id:
                # Step 3: Get the profile
                self.test_get_profile(profile_id)
                
                # Step 4: Update the profile
                update_profile_data = {
                    "profile_name": f"Updated Test Profile {int(time.time())}",
                    "description": "Profile updated by automated test"
                }
                self.test_update_profile(profile_id, update_profile_data)
                
                # Step 5: Delete the profile
                self.test_delete_profile(profile_id)
            else:
                logger.error("Failed to get profile_id from create response")
        else:
            logger.error("Profile creation failed, skipping subsequent tests")
        
        # Print summary
        self.print_results()
    
    def print_results(self) -> None:
        """Print test results in a formatted table."""
        print("\n" + "=" * 80)
        print(" BUSINESS PROFILES API TEST RESULTS".center(80))
        print("=" * 80)
        
        print(f"{'Test Name':<30} | {'Status':<10} | {'Status Code':<12} | {'Message'}")
        print("-" * 80)
        
        for result in self.test_results:
            status = "✅ PASS" if result["success"] else "❌ FAIL"
            print(f"{result['test_name']:<30} | {status:<10} | {result.get('status_code', 'N/A'):<12} | {result['message']}")
        
        # Calculate overall success rate
        success_count = sum(1 for result in self.test_results if result["success"])
        total_count = len(self.test_results)
        success_rate = (success_count / total_count) * 100 if total_count > 0 else 0
        
        print("-" * 80)
        print(f"Success Rate: {success_rate:.2f}% ({success_count}/{total_count} tests passed)")
        print("=" * 80 + "\n")

def main():
    """Main function to run the tests."""
    parser = argparse.ArgumentParser(description="Test Business Profiles API endpoints")
    parser.add_argument("--token", required=True, help="JWT token for authentication")
    parser.add_argument("--api_key", required=True, help="API key for authentication")
    parser.add_argument("--account_id", required=True, help="Account ID (merchant ID)")
    parser.add_argument("--profile_id", help="Specific profile ID to test (optional)")
    parser.add_argument("--test", choices=["all", "list", "get", "create", "update", "delete"], 
                        default="all", help="Specific test to run")
    
    args = parser.parse_args()
    
    # Initialize test class
    tester = BusinessProfilesTest(args.token, args.api_key, args.account_id)
    
    # Run specific test or all tests
    if args.test == "all":
        tester.run_all_tests()
    elif args.test == "list":
        tester.test_list_profiles()
        tester.print_results()
    elif args.test == "get" and args.profile_id:
        tester.test_get_profile(args.profile_id)
        tester.print_results()
    elif args.test == "create":
        new_profile_data = {
            "profile_name": f"Test Profile {int(time.time())}",
            "description": "Profile created by automated test"
        }
        tester.test_create_profile(new_profile_data)
        tester.print_results()
    elif args.test == "update" and args.profile_id:
        update_profile_data = {
            "profile_name": f"Updated Test Profile {int(time.time())}"
        }
        tester.test_update_profile(args.profile_id, update_profile_data)
        tester.print_results()
    elif args.test == "delete" and args.profile_id:
        tester.test_delete_profile(args.profile_id)
        tester.print_results()
    else:
        print("Error: Invalid test selection or missing required parameters")
        parser.print_help()
        return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(main()) 