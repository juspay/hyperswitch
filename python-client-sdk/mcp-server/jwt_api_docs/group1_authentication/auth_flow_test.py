#!/usr/bin/env python3
"""
Authentication Flow Test Script

This script tests the complete authentication flow for the Hyperswitch JWT API:
1. Sign-in
2. 2FA termination and JWT token acquisition
3. Get user info
4. Update user profile
5. Sign-out

Usage:
  ./auth_flow_test.py <email> <password>
"""

import sys
import json
import logging
import argparse
from datetime import datetime

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler("auth_flow_test.log"),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)

# Try to import the Hyperswitch MCP modules
try:
    from hyperswitch_mcp.auth import Auth
    from hyperswitch_mcp.user import User
except ImportError:
    logger.error("Failed to import Hyperswitch MCP modules. Make sure the package is installed.")
    logger.info("Try setting your PYTHONPATH: export PYTHONPATH=$(pwd)/../..")
    sys.exit(1)

class AuthFlowTest:
    """Test the authentication flow from sign-in to sign-out."""
    
    def __init__(self, email, password, verbose=False):
        """Initialize the test with user credentials."""
        self.email = email
        self.password = password
        self.jwt_token = None
        self.totp_token = None
        self.user_info = None
        
        if verbose:
            logging.getLogger().setLevel(logging.DEBUG)
            logger.debug("Verbose logging enabled")
    
    def run_all_tests(self):
        """Run the complete test flow."""
        try:
            logger.info("Starting authentication flow test")
            
            # Step 1: Sign in
            success = self.test_signin()
            if not success:
                logger.error("Sign-in test failed. Stopping flow.")
                return False
            
            # Step 2: Terminate 2FA
            success = self.test_terminate_2fa()
            if not success:
                logger.error("2FA termination test failed. Stopping flow.")
                return False
            
            # Step 3: Get user info
            success = self.test_get_user_info()
            if not success:
                logger.error("Get user info test failed. Stopping flow.")
                return False
            
            # Step 4: Update user profile
            success = self.test_update_profile()
            if not success:
                logger.error("Update profile test failed. Stopping flow.")
                return False
            
            # Step 5: Sign out
            success = self.test_signout()
            if not success:
                logger.error("Sign-out test failed.")
                return False
            
            logger.info("All tests passed successfully!")
            return True
            
        except Exception as e:
            logger.error(f"An unexpected error occurred: {str(e)}", exc_info=True)
            return False
    
    def test_signin(self):
        """Test the sign-in functionality."""
        logger.info("Testing sign-in...")
        
        try:
            # Create the Auth instance and sign in
            auth = Auth()
            result = auth.signin(self.email, self.password)
            
            # Log the response format for debugging
            logger.debug(f"Sign-in response: {json.dumps(result, indent=2)}")
            
            # Check if sign-in was successful
            if not result.get('success', False):
                logger.error(f"Sign-in failed: {result.get('error', 'Unknown error')}")
                return False
            
            # Extract and store the TOTP token
            self.totp_token = result.get('token')
            if not self.totp_token:
                logger.error("No TOTP token received from sign-in")
                return False
            
            logger.info("Sign-in successful, TOTP token acquired")
            return True
            
        except Exception as e:
            logger.error(f"Exception during sign-in: {str(e)}", exc_info=True)
            return False
    
    def test_terminate_2fa(self):
        """Test the 2FA termination to obtain a JWT token."""
        logger.info("Testing 2FA termination...")
        
        if not self.totp_token:
            logger.error("Cannot terminate 2FA: No TOTP token available")
            return False
        
        try:
            # Terminate 2FA to get JWT token
            auth = Auth()
            result = auth.terminate_2fa(self.totp_token, skip_two_factor_auth=True)
            
            # Log the response format for debugging
            logger.debug(f"2FA termination response: {json.dumps(result, indent=2)}")
            
            # Check if 2FA termination was successful
            if not result.get('success', False):
                logger.error(f"2FA termination failed: {result.get('error', 'Unknown error')}")
                return False
            
            # Extract and store the JWT token
            self.jwt_token = result.get('user_info_token')
            if not self.jwt_token:
                logger.error("No JWT token received from 2FA termination")
                return False
            
            logger.info("2FA termination successful, JWT token acquired")
            return True
            
        except Exception as e:
            logger.error(f"Exception during 2FA termination: {str(e)}", exc_info=True)
            return False
    
    def test_get_user_info(self):
        """Test retrieving user information using the JWT token."""
        logger.info("Testing get user info...")
        
        if not self.jwt_token:
            logger.error("Cannot get user info: No JWT token available")
            return False
        
        try:
            # Create the User instance and get user info
            user = User(self.jwt_token)
            result = user.get_info()
            
            # Log the response format for debugging
            logger.debug(f"Get user info response: {json.dumps(result, indent=2)}")
            
            # Check if getting user info was successful
            if not result.get('success', False):
                logger.error(f"Get user info failed: {result.get('error', 'Unknown error')}")
                return False
            
            # Store the user info for later use
            self.user_info = result.get('data', {})
            if not self.user_info:
                logger.warning("User info is empty or not properly structured")
            
            logger.info("Get user info successful")
            logger.info(f"User email: {self.user_info.get('email', 'N/A')}")
            logger.info(f"User name: {self.user_info.get('name', 'N/A')}")
            return True
            
        except Exception as e:
            logger.error(f"Exception during get user info: {str(e)}", exc_info=True)
            return False
    
    def test_update_profile(self):
        """Test updating the user profile."""
        logger.info("Testing update profile...")
        
        if not self.jwt_token:
            logger.error("Cannot update profile: No JWT token available")
            return False
        
        try:
            # Create the User instance and update profile with timestamp
            user = User(self.jwt_token)
            timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            
            # Get current name components
            current_name = self.user_info.get('name', 'Test User')
            name_parts = current_name.split(' ', 1)
            first_name = name_parts[0] if len(name_parts) > 0 else "Test"
            last_name = name_parts[1] if len(name_parts) > 1 else "User"
            
            # Prepare update data
            update_data = {
                'first_name': first_name,
                'last_name': f"{last_name} [{timestamp}]",
                'company_name': f"Hyperswitch Test {timestamp}"
            }
            
            logger.debug(f"Update profile data: {json.dumps(update_data, indent=2)}")
            result = user.update_profile(**update_data)
            
            # Log the response format for debugging
            logger.debug(f"Update profile response: {json.dumps(result, indent=2)}")
            
            # Check if update was successful
            if not result.get('success', False):
                logger.error(f"Update profile failed: {result.get('error', 'Unknown error')}")
                return False
            
            logger.info("Update profile successful")
            
            # Verify the updates by getting user info again
            logger.info("Verifying profile updates...")
            verify_result = user.get_info()
            
            if verify_result.get('success', False):
                updated_info = verify_result.get('data', {})
                updated_name = updated_info.get('name', '')
                logger.info(f"Updated name: {updated_name}")
                
                if last_name in updated_name and timestamp in updated_name:
                    logger.info("Profile update verified successfully")
                else:
                    logger.warning("Profile may not have been updated as expected")
            else:
                logger.warning("Could not verify profile updates")
            
            return True
            
        except Exception as e:
            logger.error(f"Exception during update profile: {str(e)}", exc_info=True)
            return False
    
    def test_signout(self):
        """Test the sign-out functionality."""
        logger.info("Testing sign-out...")
        
        if not self.jwt_token:
            logger.error("Cannot sign out: No JWT token available")
            return False
        
        try:
            # Create the Auth instance and sign out
            auth = Auth()
            result = auth.signout(self.jwt_token)
            
            # Log the response format for debugging
            logger.debug(f"Sign-out response: {json.dumps(result, indent=2)}")
            
            # Check if sign-out was successful
            if not result.get('success', False):
                logger.error(f"Sign-out failed: {result.get('error', 'Unknown error')}")
                return False
            
            logger.info("Sign-out successful")
            
            # Verify token is invalidated by trying to get user info
            try:
                logger.info("Verifying token invalidation...")
                user = User(self.jwt_token)
                verify_result = user.get_info()
                
                if not verify_result.get('success', False):
                    logger.info("Token successfully invalidated")
                else:
                    logger.warning("Token may still be valid after sign-out")
            except Exception:
                logger.info("Token successfully invalidated (exception occurred)")
            
            return True
            
        except Exception as e:
            logger.error(f"Exception during sign-out: {str(e)}", exc_info=True)
            return False


def main():
    """Parse arguments and run the tests."""
    parser = argparse.ArgumentParser(description='Test the Hyperswitch authentication flow.')
    parser.add_argument('email', help='Email address for login')
    parser.add_argument('password', help='Password for login')
    parser.add_argument('-v', '--verbose', action='store_true', help='Enable verbose output')
    args = parser.parse_args()
    
    test = AuthFlowTest(args.email, args.password, args.verbose)
    success = test.run_all_tests()
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main() 