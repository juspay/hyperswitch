#!/usr/bin/env python3
"""
Test Script for Hyperswitch MCP Tools

This script tests the MCP tools for user authentication flow in Hyperswitch.
"""

import argparse
import json
import logging
import sys
import time
import getpass # For securely getting password input if needed
import os # Add import for os

# Explicitly add current directory to path *before* imports are attempted
# This is a workaround for potential import issues when running the script directly
import os # Need os for path manipulation
sys.path.insert(0, os.getcwd()) 
# Alternatively: sys.path.insert(0, '.') # Might be simpler

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    handlers=[
        logging.FileHandler("test_tools.log", mode='w'), # Overwrite log each run
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)

# --- Tool Imports with Fallbacks --- 
# Define tool variables initially as None
say_hello = None
signin_tool = None
terminate_2fa_tool = None
get_user_info_tool = None
update_user_profile_tool = None
signout_tool = None
change_password_tool = None
initiate_password_reset_tool = None
reset_password_confirm_tool = None
verify_email_tool = None

try:
    # Try to import directly from server.py (preferred)
    from hyperswitch_mcp.server import (
        say_hello, 
        signin_tool,
        terminate_2fa_tool,
        get_user_info_tool,
        update_user_profile_tool,
        signout_tool,
        change_password_tool,
        initiate_password_reset_tool,
        reset_password_confirm_tool,
        verify_email_tool
    )
    logger.info("Successfully imported MCP tools directly from server.py")
    REAL_TOOLS_IMPORTED = True
except ImportError as e1:
    logger.warning(f"Failed to import MCP tools directly: {e1}. Trying module functions...")
    try:
        # Fall back to importing from the underlying modules
        from hyperswitch_mcp.auth import signin, terminate_2fa, signout
        from hyperswitch_mcp.user import (
            get_user_info, update_user, change_password, 
            initiate_password_reset, reset_password_confirm, verify_email
        )
        
        # Create wrapper functions with the same signatures as the tools
        def _say_hello_wrapper(name: str):
            return {"message": f"Hello, {name}!"}
        say_hello = _say_hello_wrapper
            
        def _signin_wrapper(email: str, password: str):
            return signin(email, password)
        signin_tool = _signin_wrapper
            
        def _terminate_2fa_wrapper(totp_token: str, skip_two_factor_auth: str = "True"):
            skip = skip_two_factor_auth.lower() == "true"
            result = terminate_2fa(totp_token, skip)
            # Ensure consistent output key
            if "token" in result and "error" not in result:
                result["user_info_token"] = result.pop("token")
            return result
        terminate_2fa_tool = _terminate_2fa_wrapper
            
        def _get_user_info_wrapper(jwt_token: str):
            return get_user_info(jwt_token)
        get_user_info_tool = _get_user_info_wrapper
            
        def _update_user_profile_wrapper(jwt_token: str, name: str = None, phone: str = None):
            return update_user(jwt_token, name, phone)
        update_user_profile_tool = _update_user_profile_wrapper
            
        def _signout_wrapper(jwt_token: str):
            return signout(jwt_token)
        signout_tool = _signout_wrapper

        def _change_password_wrapper(jwt_token: str, current_password: str, new_password: str):
            return change_password(jwt_token, current_password, new_password)
        change_password_tool = _change_password_wrapper

        def _initiate_reset_wrapper(email: str):
            return initiate_password_reset(email)
        initiate_password_reset_tool = _initiate_reset_wrapper

        def _confirm_reset_wrapper(reset_token: str, new_password: str):
            return reset_password_confirm(reset_token, new_password)
        reset_password_confirm_tool = _confirm_reset_wrapper

        def _verify_email_wrapper(verification_token: str):
            return verify_email(verification_token)
        verify_email_tool = _verify_email_wrapper
            
        logger.info("Successfully created wrapper functions using module imports")
        REAL_TOOLS_IMPORTED = True # Still using real logic, just via modules
    except ImportError as e2:
        logger.error(f"Failed to import module functions: {e2}. Running in MOCK mode.")
        REAL_TOOLS_IMPORTED = False
        
        # Create mock versions of ALL tools if imports failed
        def _mock_say_hello(name: str):
            logger.info(f"Mock: Saying hello to {name}")
            return {"message": f"Hello, {name}!"}
        say_hello = _mock_say_hello
            
        def _mock_signin(email: str, password: str):
            logger.info(f"Mock: Signing in with {email}")
            # Simulate success for mock testing flow
            if password == "correct_mock_password":
                return {"totp_token": "mock_totp_token_123", "user_id": "user_123", "email": email}
            else:
                return {"error": "Invalid mock credentials"}
        signin_tool = _mock_signin
            
        def _mock_terminate_2fa(totp_token: str, skip_two_factor_auth: str = "True"):
            logger.info(f"Mock: Terminating 2FA with token {totp_token[:5]}...")
            if totp_token == "mock_totp_token_123":
                return {"user_info_token": "mock_jwt_token_456", "user_id": "user_123"}
            else:
                return {"error": "Invalid mock TOTP token"}
        terminate_2fa_tool = _mock_terminate_2fa
            
        def _mock_get_user_info(jwt_token: str):
            logger.info(f"Mock: Getting user info with token {jwt_token[:5]}...")
            if jwt_token == "mock_jwt_token_456":
                return {"user_id": "user_123", "email": "test@example.com", "name": "Test User", "created_at": "2023-01-01T00:00:00Z"}
            else:
                return {"error": "Invalid mock JWT token"}
        get_user_info_tool = _mock_get_user_info
            
        def _mock_update_profile(jwt_token: str, name: str = None, phone: str = None):
            logger.info(f"Mock: Updating profile with token {jwt_token[:5]}...")
            if jwt_token == "mock_jwt_token_456":
                return {"user_id": "user_123", "email": "test@example.com", "name": name if name else "Test User", "phone": phone if phone else "+1234567890", "updated_at": "2023-01-01T00:00:00Z"}
            else:
                return {"error": "Invalid mock JWT token"}
        update_user_profile_tool = _mock_update_profile
            
        def _mock_signout(jwt_token: str):
            logger.info(f"Mock: Signing out with token {jwt_token[:5]}...")
            if jwt_token == "mock_jwt_token_456":
                return {"status": "success", "message": "Successfully signed out"}
            else: 
                return {"error": "Invalid mock JWT token"}
        signout_tool = _mock_signout
        
        def _mock_change_password(jwt_token: str, current_password: str, new_password: str):
            logger.info(f"Mock: Changing password with token {jwt_token[:5]}...")
            if jwt_token == "mock_jwt_token_456" and current_password == "correct_mock_password":
                 return {"status": "success", "message": "Password changed successfully"}
            elif jwt_token != "mock_jwt_token_456":
                return {"error": "Invalid mock JWT token"}
            else:
                return {"error": "Incorrect current password"}
        change_password_tool = _mock_change_password
        
        def _mock_initiate_reset(email: str):
            logger.info(f"Mock: Initiating password reset for {email}")
            return {"status": "success", "message": "Password reset email sent"}
        initiate_password_reset_tool = _mock_initiate_reset
        
        def _mock_confirm_reset(reset_token: str, new_password: str):
            logger.info(f"Mock: Confirming password reset with token {reset_token[:5]}...")
            if reset_token == "mock_reset_token_789":
                return {"status": "success", "message": "Password has been reset successfully"}
            else:
                return {"error": "Invalid or expired reset token"}
        reset_password_confirm_tool = _mock_confirm_reset
        
        def _mock_verify_email(verification_token: str):
            logger.info(f"Mock: Verifying email with token {verification_token[:5]}...")
            if verification_token == "mock_verify_token_abc":
                return {"status": "success", "message": "Email verified successfully"}
            else:
                return {"error": "Invalid or expired verification token"}
        verify_email_tool = _mock_verify_email
# --- End Tool Imports --- 

# --- Test Functions --- 
def test_say_hello():
    """Test the say_hello tool"""
    logger.info("--- Testing say_hello tool ---")
    name = "Test User"
    if not say_hello:
        logger.error("❌ say_hello tool not available.")
        return False
    
    result = say_hello(name)
    logger.info(f"Result: {json.dumps(result, indent=2)}")
    
    if "message" not in result or f"Hello, {name}!" not in result["message"]:
        logger.error(f"❌ say_hello test failed. Unexpected result: {result}")
        return False

    logger.info("✅ say_hello test passed")
    return True

def test_auth_flow(email: str, password: str) -> tuple[bool, str | None]:
    """Test the complete authentication flow and return status and JWT token."""
    logger.info(f"--- Testing complete authentication flow for {email} ---")
    jwt_token = None # Initialize jwt_token

    if not all([signin_tool, terminate_2fa_tool, get_user_info_tool, update_user_profile_tool, signout_tool]):
        logger.error("❌ Cannot run auth flow: One or more required tools are not available.")
        return False, None

    # Step 1: Sign in
    logger.info("Step 1: Sign in")
    signin_result = signin_tool(email, password)
    logger.info(f"Sign in result: {json.dumps(signin_result, indent=2)}")
    
    if "error" in signin_result or "totp_token" not in signin_result:
        logger.error(f"❌ Sign in failed: {signin_result.get('error', 'No TOTP token received')}")
        return False, None
        
    totp_token = signin_result.get("totp_token")
    logger.info("✅ Sign in successful")
    
    # Step 2: Terminate 2FA
    logger.info("Step 2: Terminate 2FA")
    # Assuming skip_two_factor_auth='True' for automated testing
    terminate_result = terminate_2fa_tool(totp_token, skip_two_factor_auth="True")
    logger.info(f"Terminate 2FA result: {json.dumps(terminate_result, indent=2)}")
    
    if "error" in terminate_result or "user_info_token" not in terminate_result:
        logger.error(f"❌ Terminate 2FA failed: {terminate_result.get('error', 'No JWT token received')}")
        return False, None
        
    jwt_token = terminate_result.get("user_info_token")
    logger.info("✅ Terminate 2FA successful")
    
    # Step 3: Get user info
    logger.info("Step 3: Get user info")
    userinfo_result = get_user_info_tool(jwt_token)
    logger.info(f"User info result: {json.dumps(userinfo_result, indent=2)}")
    
    if "error" in userinfo_result:
        logger.error(f"❌ Get user info failed: {userinfo_result.get('error', 'Unknown error')}")
        return False, jwt_token # Return token even if this step fails
        
    logger.info("✅ Get user info successful")
    
    # Step 4: Update profile (Optional step in basic auth flow)
    logger.info("Step 4: Update profile (basic test)")
    test_name = f"Test User {int(time.time())}"
    update_result = update_user_profile_tool(jwt_token, name=test_name)
    logger.info(f"Update profile result: {json.dumps(update_result, indent=2)}")
    
    if "error" in update_result:
        # Log warning, but don't fail the whole auth flow for this
        logger.warning(f"⚠️ Update profile failed: {update_result.get('error', 'Unknown error')}")
    else:
        logger.info("✅ Update profile successful")
    
    # Step 5: Sign out
    # We often want the token for subsequent tests, so signout is optional here
    # logger.info("Step 5: Sign out")
    # signout_result = signout_tool(jwt_token)
    # logger.info(f"Sign out result: {json.dumps(signout_result, indent=2)}")
    # 
    # if "error" in signout_result:
    #     logger.error(f"❌ Sign out failed: {signout_result.get('error', 'Unknown error')}")
    #     return False, jwt_token # Return token even if signout fails
    #     
    # logger.info("✅ Sign out successful")
    
    # Complete flow success (excluding optional sign-out)
    logger.info("✅ Core authentication flow test passed (SignOut skipped)")
    return True, jwt_token # Return success and the token

def test_change_password(jwt_token: str, current_password: str, new_password_base: str) -> tuple[bool, str | None]:
    """Test the change_password tool. Returns status and the new password if successful."""
    logger.info("--- Testing Change Password --- ")
    # Ensure new password is different using a timestamp
    new_password_attempt = f"{new_password_base}_{int(time.time())}" 

    if not change_password_tool:
        logger.error("❌ change_password_tool not available.")
        return False, None
    if not jwt_token:
        logger.error("❌ Cannot change password without a valid JWT token.")
        return False, None

    logger.info(f"Attempting to change password from '{current_password[:1]}...{current_password[-1:]}' to '{new_password_attempt[:1]}...{new_password_attempt[-1:]}'")
    result = change_password_tool(
        jwt_token=jwt_token, 
        current_password=current_password, 
        new_password=new_password_attempt
    )
    logger.info(f"Change password result: {json.dumps(result, indent=2)}")

    if "error" in result or result.get("status") != "success":
        logger.error(f"❌ Change password failed: {result.get('error', 'Unknown error')}")
        return False, None
    
    logger.info("✅ Change password successful")
    # Ideally, we would try logging in with the new password here, 
    # but that complicates the test flow state.
    return True, new_password_attempt # Return success and the actual new password used

def test_password_reset(email: str, new_password: str) -> bool:
    """Test the password reset flow (initiate and confirm)."""
    logger.info(f"--- Testing Password Reset Flow for {email} ---")
    if not initiate_password_reset_tool or not reset_password_confirm_tool:
        logger.error("❌ Password reset tools not available.")
        return False

    # Step 1: Initiate Reset
    logger.info("Step 1: Initiate password reset")
    initiate_result = initiate_password_reset_tool(email=email)
    logger.info(f"Initiate reset result: {json.dumps(initiate_result, indent=2)}")

    if "error" in initiate_result or initiate_result.get("status") != "success":
         logger.error(f"❌ Initiate password reset failed: {initiate_result.get('error', 'Unknown error')}")
         return False
    
    logger.info("✅ Initiate password reset successful (assuming email sent)")

    # Step 2: Confirm Reset (Requires manual token extraction)
    logger.info("Step 2: Confirm password reset")
    logger.warning("❗ This step requires manual intervention.")
    logger.warning("❗ Please check the email for the reset token and paste it below.")
    
    reset_token = input("Enter password reset token: ").strip()
    
    if not reset_token:
        logger.error("❌ No reset token provided. Skipping confirm reset test.")
        return False # Consider this failure or skip?
        
    confirm_result = reset_password_confirm_tool(
        reset_token=reset_token,
        new_password=new_password
    )
    logger.info(f"Confirm reset result: {json.dumps(confirm_result, indent=2)}")

    if "error" in confirm_result or confirm_result.get("status") != "success":
         logger.error(f"❌ Confirm password reset failed: {confirm_result.get('error', 'Unknown error')}")
         return False

    logger.info("✅ Confirm password reset successful")
    # We should attempt login with the new password after this.
    return True

def test_email_verification(jwt_token: str) -> bool:
    """Test the email verification flow (confirm step only)."""
    logger.info("--- Testing Email Verification --- ")
    if not verify_email_tool:
        logger.error("❌ verify_email_tool not available.")
        return False
    if not jwt_token:
        logger.error("❌ Cannot verify email without JWT token.")
        return False

    logger.warning("❗ This test requires manual intervention.")
    logger.warning("❗ Please trigger email verification (e.g., during signup or manually) ")
    logger.warning("❗ and paste the verification token below.")

    verification_token = input("Enter email verification token: ").strip()
    
    if not verification_token:
        logger.error("❌ No verification token provided. Skipping email verification test.")
        return False
        
    result = verify_email_tool(jwt_token=jwt_token, verification_token=verification_token)
    logger.info(f"Email verification result: {json.dumps(result, indent=2)}")

    if "error" in result or result.get("status") != "success":
         logger.error(f"❌ Email verification failed: {result.get('error', 'Unknown error')}")
         return False

    logger.info("✅ Email verification successful")
    return True

# --- Main Execution Logic --- 
def main():
    """Main function to parse args and run tests."""
    parser = argparse.ArgumentParser(description="Test Hyperswitch MCP Tools")
    parser.add_argument("--email", required=True, help="Email for authentication")
    # Use getpass for password security if not provided
    parser.add_argument("--password", help="Password for authentication (will prompt if not provided)") 
    parser.add_argument("--new-password", default="NewPa$$w0rd123", help="New password for change/reset tests")
    parser.add_argument("--test", choices=["hello", "auth", "change_pwd", "reset_pwd", "verify_email", "all"], default="all", 
                        help="Test to run (hello, auth, change_pwd, reset_pwd, verify_email, or all)")
    args = parser.parse_args()
    
    # Get password securely if not provided
    password = args.password
    if not password:
        password = getpass.getpass(f"Enter password for {args.email}: ")

    overall_success = True
    jwt_token = None # To store token from auth flow
    current_password = password # Keep track of the current password
    
    logger.info("Starting MCP tools tests")
    if not REAL_TOOLS_IMPORTED:
        logger.warning("⚠️ Running in MOCK mode. Tests may not reflect real API behavior.")
        # Use mock credentials for mock mode
        current_password = "correct_mock_password"

    if args.test in ["hello", "all"]:
        try:
            result = test_say_hello()
            overall_success = result and overall_success
        except Exception as e:
            logger.exception(f"❌ say_hello test failed with exception: {e}")
            overall_success = False
    
    # Auth flow is needed for subsequent tests that require JWT
    if args.test in ["auth", "change_pwd", "all"]:
        logger.info("--- Running Auth Flow to get JWT --- ")
        try:
            auth_success, jwt_token = test_auth_flow(args.email, current_password)
            overall_success = auth_success and overall_success
            if not auth_success:
                 logger.error("Auth flow failed, cannot proceed with JWT-dependent tests.")
                 # Exit early if auth fails and other tests depend on it
                 if args.test in ["change_pwd", "all"]:
                    sys.exit(1)
            elif not jwt_token:
                logger.error("Auth flow succeeded but did not return JWT token. Cannot proceed.")
                if args.test in ["change_pwd", "all"]:
                    sys.exit(1)
        except Exception as e:
            logger.exception(f"❌ auth flow test failed with exception: {e}")
            overall_success = False
            # Exit early if auth fails and other tests depend on it
            if args.test in ["change_pwd", "all"]:
                sys.exit(1)
    
    if args.test in ["change_pwd", "all"]:
        if jwt_token:
            try:
                # Pass the base new password from args
                change_success, actual_new_password = test_change_password(jwt_token, current_password, args.new_password) 
                overall_success = change_success and overall_success
                # If change was successful, update the password for potential subsequent tests
                if change_success:
                    current_password = actual_new_password # Use the password that was actually set
                    # Ideally, sign out and sign back in with new password here
            except Exception as e:
                logger.exception(f"❌ change_password test failed with exception: {e}")
                overall_success = False
        else:
            logger.error("❌ Skipping change_password test: No valid JWT token obtained.")
            overall_success = False

    if args.test in ["reset_pwd", "all"]:
         try:
             # Use the base new password from args for reset target
             reset_success = test_password_reset(args.email, args.new_password)
             overall_success = reset_success and overall_success
             # If reset was successful, update password state
             if reset_success:
                 current_password = args.new_password # Reset sets to the base new password
         except Exception as e:
             logger.exception(f"❌ password_reset test failed with exception: {e}")
             overall_success = False

    if args.test in ["verify_email", "all"]:
         if not jwt_token:
             logger.error("❌ Skipping email_verification test: No valid JWT token obtained.")
             overall_success = False
         else:
            try:
                # Pass jwt_token to the test function
                verify_success = test_email_verification(jwt_token)
                overall_success = verify_success and overall_success
            except Exception as e:
                logger.exception(f"❌ email_verification test failed with exception: {e}")
                overall_success = False

    # Final sign out if a token exists (useful if not testing signout explicitly earlier)
    # Optional: Add a specific test case for signout if needed
    # if jwt_token and signout_tool:
    #     logger.info("--- Performing Final Sign Out ---")
    #     signout_tool(jwt_token)

    logger.info("--- Test Run Summary ---")
    if overall_success:
        logger.info("✅ All executed tests passed successfully!")
        return 0
    else:
        logger.error("❌ Some tests failed. See log for details.")
        return 1

if __name__ == "__main__":
    # Ensure hyperswitch_mcp is in the Python path if running directly
    # import os # Add import for os
    # Add the parent directory (python-client-sdk) to sys.path
    # sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../python-client-sdk')))
    # Remove the old path insert which added mcp-server directly
    # sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '../python-client-sdk/mcp-server')))
    sys.exit(main()) 