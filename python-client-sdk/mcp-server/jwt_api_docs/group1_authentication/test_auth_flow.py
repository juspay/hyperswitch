#!/usr/bin/env python3

"""
Hyperswitch JWT Authentication Flow Test Script

This script tests the complete JWT authentication flow including:
1. Sign in with credentials
2. Terminate 2FA (if enabled)
3. Get user info using JWT
4. Update user profile
5. Sign out

Usage:
    python test_auth_flow.py [--verbose]
"""

import argparse
import json
import sys
import time
from typing import Dict, Any, Optional, Tuple

# Import MCP tools for Hyperswitch
try:
    from mcp_hyperswitch_user_based_flow_Sign_in_to_Hyperswitch import sign_in
    from mcp_hyperswitch_user_based_flow_Terminate_2FA import terminate_2fa
    from mcp_hyperswitch_user_based_flow_Get_User_Info import get_user_info
    from mcp_hyperswitch_user_based_flow_Update_User_Profile import update_profile
    from mcp_hyperswitch_user_based_flow_Sign_out_from_Hyperswitch import sign_out
except ImportError:
    print("Error: Required MCP modules not found. Make sure you're running in the correct environment.")
    sys.exit(1)

# Configuration
DEFAULT_EMAIL = "test@example.com"  # Replace with test account
DEFAULT_PASSWORD = "TestPassword123"  # Replace with test password


class AuthFlowTester:
    """Tests the complete authentication flow for Hyperswitch JWT APIs"""

    def __init__(self, verbose: bool = False):
        self.verbose = verbose
        self.email = DEFAULT_EMAIL
        self.password = DEFAULT_PASSWORD
        self.jwt_token = None
        self.user_id = None
        self.success_count = 0
        self.failure_count = 0
        self.start_time = time.time()

    def log(self, message: str, level: str = "INFO"):
        """Log messages if verbose mode is enabled"""
        if self.verbose:
            print(f"[{level}] {message}")

    def log_response(self, step: str, response: Dict[str, Any]):
        """Log API response details"""
        if self.verbose:
            print(f"\n=== {step} Response ===")
            print(json.dumps(response, indent=2))
            print("=" * 50)

    def run_step(self, step_name: str, func, *args, **kwargs) -> Tuple[bool, Optional[Dict[str, Any]]]:
        """Run a test step and handle errors"""
        print(f"Running: {step_name}...")
        try:
            result = func(*args, **kwargs)
            self.log_response(step_name, result)
            self.success_count += 1
            print(f"✅ {step_name} - Success")
            return True, result
        except Exception as e:
            self.failure_count += 1
            print(f"❌ {step_name} - Failed: {str(e)}")
            self.log(f"Exception details: {repr(e)}", "ERROR")
            return False, None

    def test_sign_in(self) -> bool:
        """Test sign in and retrieve JWT token"""
        success, result = self.run_step(
            "Sign In", 
            sign_in, 
            email=self.email, 
            password=self.password
        )
        
        if success and result:
            self.jwt_token = result.get("token")
            self.user_id = result.get("user_id")
            
            if not self.jwt_token:
                print("❌ No JWT token found in sign-in response")
                return False
                
            self.log(f"JWT Token received: {self.jwt_token[:10]}...")
            self.log(f"User ID: {self.user_id}")
            return True
        return False

    def test_terminate_2fa(self) -> bool:
        """Test 2FA termination (if required)"""
        if not self.jwt_token:
            return False
            
        # In a real scenario, you would get the 2FA code from the user or a test authenticator
        test_2fa_code = "123456"  # This would be a valid code in a real test
        
        success, result = self.run_step(
            "Terminate 2FA", 
            terminate_2fa, 
            token=self.jwt_token, 
            totp_code=test_2fa_code
        )
        
        if success and result:
            # Update token if a new one is returned after 2FA
            new_token = result.get("token")
            if new_token:
                self.jwt_token = new_token
                self.log(f"Updated JWT Token after 2FA: {self.jwt_token[:10]}...")
            return True
        return False

    def test_get_user_info(self) -> bool:
        """Test retrieving user information using JWT token"""
        if not self.jwt_token:
            return False
            
        success, result = self.run_step(
            "Get User Info", 
            get_user_info, 
            token=self.jwt_token
        )
        
        if success and result:
            self.log(f"User email: {result.get('email')}")
            self.log(f"User name: {result.get('name')}")
            return True
        return False

    def test_update_profile(self) -> bool:
        """Test updating user profile"""
        if not self.jwt_token:
            return False
            
        # Generate a unique test name to verify the update worked
        test_name = f"Test User {int(time.time()) % 10000}"
        
        success, result = self.run_step(
            "Update Profile", 
            update_profile, 
            token=self.jwt_token, 
            name=test_name
        )
        
        if success:
            # Verify the update by getting user info again
            verify_success, verify_result = self.run_step(
                "Verify Profile Update", 
                get_user_info, 
                token=self.jwt_token
            )
            
            if verify_success and verify_result and verify_result.get("name") == test_name:
                print(f"✅ Profile update verified - name changed to '{test_name}'")
                return True
            else:
                print("❌ Profile update could not be verified")
                return False
        return False

    def test_sign_out(self) -> bool:
        """Test signing out"""
        if not self.jwt_token:
            return False
            
        success, result = self.run_step(
            "Sign Out", 
            sign_out, 
            token=self.jwt_token
        )
        
        if success:
            # Try to use the token after logout - should fail
            try:
                get_user_info(token=self.jwt_token)
                print("❌ Token still valid after logout")
                return False
            except Exception as e:
                self.log(f"Expected error after logout: {str(e)}")
                print("✅ Token invalidated after logout")
                return True
        return False

    def run_all_tests(self):
        """Run the complete authentication flow test"""
        print("\n=== Hyperswitch JWT Authentication Flow Test ===\n")
        
        # Step 1: Sign In
        if not self.test_sign_in():
            print("\n❌ Authentication flow test failed at sign-in step")
            self.print_summary()
            return False
            
        # Step 2: Terminate 2FA (may be skipped if 2FA not required)
        self.test_terminate_2fa()
        
        # Step 3: Get User Info
        if not self.test_get_user_info():
            print("\n❌ Authentication flow test failed at get user info step")
            self.print_summary()
            return False
            
        # Step 4: Update Profile
        if not self.test_update_profile():
            print("\n❌ Authentication flow test failed at update profile step")
            self.print_summary()
            return False
            
        # Step 5: Sign Out
        if not self.test_sign_out():
            print("\n❌ Authentication flow test failed at sign-out step")
            self.print_summary()
            return False
            
        print("\n✅ Authentication flow test completed successfully")
        self.print_summary()
        return True

    def print_summary(self):
        """Print test summary"""
        duration = time.time() - self.start_time
        print("\n=== Test Summary ===")
        print(f"Total tests: {self.success_count + self.failure_count}")
        print(f"Successful: {self.success_count}")
        print(f"Failed: {self.failure_count}")
        print(f"Duration: {duration:.2f} seconds")
        print("=" * 50)


def main():
    parser = argparse.ArgumentParser(description="Test Hyperswitch JWT Authentication Flow")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose output")
    parser.add_argument("--email", help="Email for testing (defaults to test account)")
    parser.add_argument("--password", help="Password for testing (defaults to test account)")
    
    args = parser.parse_args()
    
    tester = AuthFlowTester(verbose=args.verbose)
    
    # Override defaults if provided
    if args.email:
        tester.email = args.email
    if args.password:
        tester.password = args.password
        
    # Run all tests
    success = tester.run_all_tests()
    
    # Exit with appropriate code
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main() 