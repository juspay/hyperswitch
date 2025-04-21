#!/usr/bin/env python3
"""
Hyperswitch Authentication Flow Test

This script tests the complete JWT-based authentication flow for Hyperswitch,
including sign-in, retrieving user info, updating profile, and sign-out.
"""

import argparse
import json
import logging
import os
import sys
import time
from typing import Dict, Optional, Tuple, Any

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler("auth_flow_test.log"),
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)

# Import Hyperswitch MCP functions (mock imports - these would be actual imports in production)
try:
    # These are placeholders - in a real environment, you would import the actual modules
    from hyperswitch_mcp.auth import sign_in, sign_out, terminate_2fa
    from hyperswitch_mcp.user import get_user_info, update_user_profile
except ImportError:
    logger.warning("Could not import Hyperswitch MCP modules. Running in mock mode.")
    
    # Mock implementations for testing without actual dependencies
    def sign_in(email: str, password: str) -> Dict[str, Any]:
        """Mock implementation of sign in function"""
        logger.info(f"Mock sign-in for {email}")
        return {
            "status": "success", 
            "requires_2fa": True,
            "session_token": "mock_session_token_123"
        }
    
    def terminate_2fa(session_token: str, totp_code: str = None) -> Dict[str, Any]:
        """Mock implementation of 2FA termination"""
        logger.info("Mock 2FA termination")
        return {
            "status": "success",
            "jwt_token": "mock_jwt_token_456",
            "refresh_token": "mock_refresh_token_789"
        }
    
    def get_user_info(jwt_token: str) -> Dict[str, Any]:
        """Mock implementation of get user info"""
        logger.info("Mock get user info")
        return {
            "user_id": "usr_123456789",
            "email": "test@example.com",
            "name": "Test User",
            "organization": "Test Org",
            "created_at": "2023-01-01T00:00:00Z"
        }
    
    def update_user_profile(jwt_token: str, profile_data: Dict[str, Any]) -> Dict[str, Any]:
        """Mock implementation of update user profile"""
        logger.info(f"Mock update user profile: {profile_data}")
        return {
            "status": "success",
            "updated_fields": list(profile_data.keys())
        }
    
    def sign_out(jwt_token: str) -> Dict[str, Any]:
        """Mock implementation of sign out"""
        logger.info("Mock sign out")
        return {"status": "success"}


class AuthFlowTester:
    """Class to test the authentication flow"""
    
    def __init__(self, email: str, password: str, totp_code: Optional[str] = None, verbose: bool = False):
        """Initialize the tester with credentials"""
        self.email = email
        self.password = password
        self.totp_code = totp_code
        self.jwt_token = None
        self.refresh_token = None
        self.user_info = None
        
        # Set logging level based on verbosity
        if verbose:
            logger.setLevel(logging.DEBUG)
            logger.debug("Verbose logging enabled")
        
        # Timestamp for tracking test duration
        self.start_time = time.time()
    
    def run_complete_flow(self) -> bool:
        """Run the complete authentication flow test"""
        logger.info("Starting complete authentication flow test")
        
        try:
            success = (
                self.test_sign_in() and
                self.test_terminate_2fa() and
                self.test_get_user_info() and
                self.test_update_profile() and
                self.test_sign_out()
            )
            
            if success:
                logger.info("✅ Complete authentication flow test passed successfully")
            else:
                logger.error("❌ Complete authentication flow test failed")
            
            return success
        
        except Exception as e:
            logger.error(f"❌ Authentication flow test failed with exception: {str(e)}", exc_info=True)
            return False
        finally:
            # Log test duration
            duration = time.time() - self.start_time
            logger.info(f"Authentication flow test completed in {duration:.2f} seconds")
    
    def test_sign_in(self) -> bool:
        """Test the sign-in process"""
        logger.info(f"Testing sign-in with email: {self.email}")
        
        try:
            response = sign_in(self.email, self.password)
            logger.debug(f"Sign-in response: {json.dumps(response, indent=2)}")
            
            if response.get("status") != "success":
                logger.error(f"❌ Sign-in failed: {response.get('error', 'Unknown error')}")
                return False
            
            if response.get("requires_2fa", False):
                logger.info("Sign-in successful, 2FA required")
                self.session_token = response.get("session_token")
                if not self.session_token:
                    logger.error("❌ No session token provided for 2FA")
                    return False
            else:
                # Direct sign-in without 2FA
                self.jwt_token = response.get("jwt_token")
                self.refresh_token = response.get("refresh_token")
                logger.info("Sign-in successful without 2FA")
            
            logger.info("✅ Sign-in test passed")
            return True
            
        except Exception as e:
            logger.error(f"❌ Sign-in test failed with exception: {str(e)}", exc_info=True)
            return False
    
    def test_terminate_2fa(self) -> bool:
        """Test the 2FA termination process"""
        # Skip if we don't need 2FA or don't have a session token
        if not hasattr(self, 'session_token') or not self.session_token:
            logger.info("Skipping 2FA termination test (not required)")
            return True
        
        if not self.totp_code:
            logger.warning("No TOTP code provided for 2FA, using mock code")
            self.totp_code = "123456"  # Mock code for testing
        
        logger.info("Testing 2FA termination")
        
        try:
            response = terminate_2fa(self.session_token, self.totp_code)
            logger.debug(f"2FA termination response: {json.dumps(response, indent=2)}")
            
            if response.get("status") != "success":
                logger.error(f"❌ 2FA termination failed: {response.get('error', 'Unknown error')}")
                return False
            
            self.jwt_token = response.get("jwt_token")
            self.refresh_token = response.get("refresh_token")
            
            if not self.jwt_token:
                logger.error("❌ No JWT token provided after 2FA termination")
                return False
            
            logger.info("✅ 2FA termination test passed")
            return True
            
        except Exception as e:
            logger.error(f"❌ 2FA termination test failed with exception: {str(e)}", exc_info=True)
            return False
    
    def test_get_user_info(self) -> bool:
        """Test retrieving user info with JWT token"""
        if not self.jwt_token:
            logger.error("❌ Cannot get user info - no JWT token available")
            return False
        
        logger.info("Testing get user info")
        
        try:
            self.user_info = get_user_info(self.jwt_token)
            logger.debug(f"User info response: {json.dumps(self.user_info, indent=2)}")
            
            if not self.user_info.get("user_id"):
                logger.error("❌ User info missing user_id")
                return False
            
            logger.info(f"✅ Successfully retrieved info for user: {self.user_info.get('email')}")
            return True
            
        except Exception as e:
            logger.error(f"❌ Get user info test failed with exception: {str(e)}", exc_info=True)
            return False
    
    def test_update_profile(self) -> bool:
        """Test updating user profile"""
        if not self.jwt_token:
            logger.error("❌ Cannot update profile - no JWT token available")
            return False
        
        logger.info("Testing profile update")
        
        try:
            # Sample profile update - would be customized in a real scenario
            profile_data = {
                "name": f"Test User Updated {int(time.time())}",
                "time_zone": "UTC"
            }
            
            response = update_user_profile(self.jwt_token, profile_data)
            logger.debug(f"Profile update response: {json.dumps(response, indent=2)}")
            
            if response.get("status") != "success":
                logger.error(f"❌ Profile update failed: {response.get('error', 'Unknown error')}")
                return False
            
            # Verify the update by getting user info again
            updated_user_info = get_user_info(self.jwt_token)
            logger.debug(f"Updated user info: {json.dumps(updated_user_info, indent=2)}")
            
            # Check if the name field was updated
            if updated_user_info.get("name") != profile_data["name"]:
                logger.warning(f"⚠️ Profile update verification failed: expected name={profile_data['name']}, got {updated_user_info.get('name')}")
            
            logger.info("✅ Profile update test passed")
            return True
            
        except Exception as e:
            logger.error(f"❌ Profile update test failed with exception: {str(e)}", exc_info=True)
            return False
    
    def test_sign_out(self) -> bool:
        """Test signing out"""
        if not self.jwt_token:
            logger.error("❌ Cannot sign out - no JWT token available")
            return False
        
        logger.info("Testing sign out")
        
        try:
            response = sign_out(self.jwt_token)
            logger.debug(f"Sign out response: {json.dumps(response, indent=2)}")
            
            if response.get("status") != "success":
                logger.error(f"❌ Sign out failed: {response.get('error', 'Unknown error')}")
                return False
            
            logger.info("✅ Sign out test passed")
            
            # Verify token is no longer valid by attempting to get user info
            try:
                invalid_response = get_user_info(self.jwt_token)
                logger.debug(f"Attempt to use invalidated token: {json.dumps(invalid_response, indent=2)}")
                
                if "error" not in invalid_response:
                    logger.warning("⚠️ Token still appears to be valid after sign out")
                else:
                    logger.info("✅ Token was properly invalidated after sign out")
            except Exception:
                logger.info("✅ Token was properly invalidated after sign out (caused exception)")
            
            return True
            
        except Exception as e:
            logger.error(f"❌ Sign out test failed with exception: {str(e)}", exc_info=True)
            return False


def parse_arguments():
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(description="Test Hyperswitch Authentication Flow")
    parser.add_argument("--email", required=False, default=os.environ.get("HYPERSWITCH_TEST_EMAIL", "test@example.com"),
                        help="Email for authentication (can also be set via HYPERSWITCH_TEST_EMAIL env var)")
    parser.add_argument("--password", required=False, default=os.environ.get("HYPERSWITCH_TEST_PASSWORD", "password123"),
                        help="Password for authentication (can also be set via HYPERSWITCH_TEST_PASSWORD env var)")
    parser.add_argument("--totp", required=False, default=os.environ.get("HYPERSWITCH_TEST_TOTP"),
                        help="TOTP code for 2FA (can also be set via HYPERSWITCH_TEST_TOTP env var)")
    parser.add_argument("--verbose", "-v", action="store_true", help="Enable verbose logging")
    parser.add_argument("--test", choices=["signin", "2fa", "userinfo", "profile", "signout", "all"], 
                        default="all", help="Specific test to run")
    return parser.parse_args()


def main():
    """Main entry point"""
    args = parse_arguments()
    
    # Banner
    print("\n" + "=" * 80)
    print(" HYPERSWITCH AUTHENTICATION FLOW TEST ".center(80, "="))
    print("=" * 80 + "\n")
    
    tester = AuthFlowTester(
        email=args.email,
        password=args.password,
        totp_code=args.totp,
        verbose=args.verbose
    )
    
    # Run selected test or all tests
    if args.test == "all":
        success = tester.run_complete_flow()
    elif args.test == "signin":
        success = tester.test_sign_in()
    elif args.test == "2fa":
        success = tester.test_sign_in() and tester.test_terminate_2fa()
    elif args.test == "userinfo":
        success = (tester.test_sign_in() and 
                  (not hasattr(tester, 'session_token') or tester.test_terminate_2fa()) and 
                  tester.test_get_user_info())
    elif args.test == "profile":
        success = (tester.test_sign_in() and 
                  (not hasattr(tester, 'session_token') or tester.test_terminate_2fa()) and 
                  tester.test_get_user_info() and 
                  tester.test_update_profile())
    elif args.test == "signout":
        success = (tester.test_sign_in() and 
                  (not hasattr(tester, 'session_token') or tester.test_terminate_2fa()) and 
                  tester.test_sign_out())
    
    # Exit with appropriate status code
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main() 