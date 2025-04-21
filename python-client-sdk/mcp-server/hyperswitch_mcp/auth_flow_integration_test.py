#!/usr/bin/env python3
"""
Authentication Flow Test Script

This script tests the complete authentication flow of the Hyperswitch JWT-based API system:
1. Sign in with email/password
2. Terminate 2FA to get JWT token
3. Get user information using JWT token
4. Update user profile
5. Sign out

Usage:
    python test_auth_flow.py <email> <password>
"""

import sys
import json
import time
from typing import Dict, Any, Tuple, List, Optional

# Import our authentication and user modules
from hyperswitch_mcp.auth import signin, terminate_2fa, signout
from hyperswitch_mcp.user import get_user_info, update_user
from hyperswitch_mcp.utils import Logger, LogLevel, initialize_logging

# Initialize colored output for better readability
try:
    from colorama import init, Fore, Style
    init()  # Initialize colorama
    COLOR_ENABLED = True
except ImportError:
    # If colorama is not available, define dummy color constants
    class DummyFore:
        GREEN = ""
        RED = ""
        YELLOW = ""
        BLUE = ""
        MAGENTA = ""
        CYAN = ""
    
    class DummyStyle:
        BRIGHT = ""
        RESET_ALL = ""
    
    Fore = DummyFore()
    Style = DummyStyle()
    COLOR_ENABLED = False

# Test results tracking
tests_passed = 0
tests_failed = 0
test_results = []

def print_header(title: str) -> None:
    """Print a formatted header for test sections."""
    print("\n" + "=" * 80)
    print(f"{Fore.CYAN}{Style.BRIGHT}{title}{Style.RESET_ALL}")
    print("=" * 80)

def print_step(step: str) -> None:
    """Print a formatted step in the test process."""
    print(f"\n{Fore.BLUE}➤ {step}{Style.RESET_ALL}")

def print_success(message: str) -> None:
    """Print a success message."""
    print(f"{Fore.GREEN}✓ {message}{Style.RESET_ALL}")

def print_failure(message: str) -> None:
    """Print a failure message."""
    print(f"{Fore.RED}✗ {message}{Style.RESET_ALL}")

def print_warning(message: str) -> None:
    """Print a warning message."""
    print(f"{Fore.YELLOW}⚠ {message}{Style.RESET_ALL}")

def print_info(message: str) -> None:
    """Print an informational message."""
    print(f"{Fore.CYAN}ℹ {message}{Style.RESET_ALL}")

def print_data(label: str, data: Any) -> None:
    """Print formatted data."""
    if isinstance(data, dict) or isinstance(data, list):
        formatted_data = json.dumps(data, indent=2)
        print(f"{Fore.MAGENTA}{label}:{Style.RESET_ALL}\n{formatted_data}")
    else:
        print(f"{Fore.MAGENTA}{label}:{Style.RESET_ALL} {data}")

# Rename test_step to avoid pytest collection
def _track_test_step(description: str, expected_result: str) -> callable:
    """Decorator for test steps to track results."""
    def decorator(func):
        def wrapper(*args, **kwargs):
            global tests_passed, tests_failed
            
            print_step(description)
            start_time = time.time()
            
            try:
                result = func(*args, **kwargs)
                end_time = time.time()
                duration = end_time - start_time
                
                if "error" in result:
                    tests_failed += 1
                    print_failure(f"Test failed in {duration:.2f}s: {result.get('error', 'Unknown error')}")
                    if "details" in result:
                        print_data("Error details", result["details"])
                    test_results.append({
                        "test": description,
                        "result": "FAIL",
                        "error": result.get("error"),
                        "duration": duration
                    })
                    return result
                else:
                    tests_passed += 1
                    print_success(f"Test passed in {duration:.2f}s: {expected_result}")
                    test_results.append({
                        "test": description,
                        "result": "PASS",
                        "duration": duration
                    })
                    return result
            except Exception as e:
                end_time = time.time()
                duration = end_time - start_time
                tests_failed += 1
                print_failure(f"Test raised exception in {duration:.2f}s: {str(e)}")
                test_results.append({
                    "test": description,
                    "result": "ERROR",
                    "exception": str(e),
                    "duration": duration
                })
                return {"error": f"Exception: {str(e)}"}
                
        return wrapper
    return decorator

# Rename test functions to start with _step_ to avoid pytest collection
@_track_test_step("Sign in with email and password", "Successfully signed in and obtained TOTP token")
def _step_signin(email: str, password: str) -> Dict[str, Any]:
    """Test signing in with email and password."""
    result = signin(email, password)
    
    if "error" not in result:
        if "token" in result:
            # For consistent naming across our tests
            result["totp_token"] = result.pop("token")
            
        print_data("User ID", result.get("user_id", "Not provided"))
        print_data("Email", result.get("email", "Not provided"))
        print_data("TOTP Token Prefix", result.get("totp_token", "")[:10] + "..." if result.get("totp_token") else "Not provided")
    
    return result

@_track_test_step("Terminate 2FA and get JWT token", "Successfully obtained JWT token")
def _step_terminate_2fa(totp_token: str) -> Dict[str, Any]:
    """Test terminating 2FA to get JWT token."""
    result = terminate_2fa(totp_token, skip_two_factor_auth=True)
    
    if "error" not in result:
        if "token" in result:
            # For consistent naming
            result["user_info_token"] = result.pop("token")
            
        print_data("User ID", result.get("user_id", "Not provided"))
        print_data("JWT Token Prefix", result.get("user_info_token", "")[:15] + "..." if result.get("user_info_token") else "Not provided")
    
    return result

@_track_test_step("Get user information", "Successfully retrieved user information")
def _step_get_user_info(jwt_token: str) -> Dict[str, Any]:
    """Test getting user information with JWT token."""
    result = get_user_info(jwt_token)
    
    if "error" not in result:
        print_data("User ID", result.get("user_id", "Not provided"))
        print_data("Email", result.get("email", "Not provided"))
        print_data("Name", result.get("name", "Not provided"))
        
        # Filter out sensitive or verbose fields for display
        display_fields = ["user_id", "email", "name", "created_at", "roles", "status"]
        display_data = {k: v for k, v in result.items() if k in display_fields}
        print_data("User Profile", display_data)
    
    return result

@_track_test_step("Update user profile", "Successfully updated user profile")
def _step_update_user(jwt_token: str, name: str = None, phone: str = None) -> Dict[str, Any]:
    """Test updating user profile with JWT token."""
    # Prepare update fields
    update_fields = {}
    if name is not None:
        update_fields["name"] = name
    if phone is not None:
        update_fields["phone"] = phone
    
    print_data("Update Fields", update_fields)
    
    result = update_user(jwt_token, name, phone)
    
    if "error" not in result:
        # Show updated fields
        if name is not None:
            print_data("Updated Name", result.get("name", "Not provided"))
        if phone is not None:
            print_data("Updated Phone", result.get("phone", "Not provided"))
        
        # Filter out sensitive or verbose fields for display
        display_fields = ["user_id", "email", "name", "phone", "updated_at"]
        display_data = {k: v for k, v in result.items() if k in display_fields}
        print_data("Updated Profile", display_data)
    
    return result

@_track_test_step("Sign out", "Successfully signed out")
def _step_signout(jwt_token: str) -> Dict[str, Any]:
    """Test signing out with JWT token."""
    result = signout(jwt_token)
    
    if "error" not in result:
        print_data("Status", result.get("status", "Not provided"))
        print_data("Message", result.get("message", "Not provided"))
    
    return result

def run_all_tests(email: str, password: str) -> None:
    """Run all tests in sequence."""
    print_header("HYPERSWITCH AUTHENTICATION FLOW TEST")
    print_info(f"Testing authentication flow for user: {email}")
    
    # Test 1: Sign in
    signin_result = _step_signin(email, password)
    if "error" in signin_result:
        print_warning("Sign in failed. Cannot proceed with further tests.")
        return
    
    totp_token = signin_result.get("totp_token")
    if not totp_token:
        print_warning("TOTP token not found in sign in response. Cannot proceed with further tests.")
        return
    
    # Test 2: Terminate 2FA
    terminate_result = _step_terminate_2fa(totp_token)
    if "error" in terminate_result:
        print_warning("2FA termination failed. Cannot proceed with further tests.")
        return
    
    jwt_token = terminate_result.get("user_info_token")
    if not jwt_token:
        print_warning("JWT token not found in 2FA termination response. Cannot proceed with further tests.")
        return
    
    # Test 3: Get user info
    user_info_result = test_get_user_info(jwt_token)
    if "error" in user_info_result:
        print_warning("Getting user info failed. Continuing with next tests.")
    
    # Test 4: Update user
    # Generate a unique test name based on timestamp to avoid conflicts
    test_name = f"Test User {int(time.time())}"
    update_result = test_update_user(jwt_token, name=test_name)
    if "error" in update_result:
        print_warning("Updating user failed. Continuing with next tests.")
    
    # Test 5: Sign out
    signout_result = test_signout(jwt_token)
    if "error" in signout_result:
        print_warning("Sign out failed.")
    
    # Verify token is actually invalidated by trying to use it again
    print_step("Verifying token invalidation")
    verification_result = get_user_info(jwt_token)
    if "error" in verification_result:
        print_success("JWT token was successfully invalidated")
    else:
        print_failure("JWT token is still valid after sign out")
        tests_failed += 1
    
    # Print summary
    print_header("TEST SUMMARY")
    print_info(f"Tests Passed: {tests_passed}")
    print_info(f"Tests Failed: {tests_failed}")
    
    if tests_failed == 0:
        print_success("All tests passed successfully!")
    else:
        print_failure(f"{tests_failed} test(s) failed. See details above.")

def main():
    """Main function to run the test script."""
    # Set up logging
    initialize_logging(LogLevel.INFO)
    
    # Check command line arguments
    if len(sys.argv) < 3:
        print("Usage: python test_auth_flow.py <email> <password>")
        sys.exit(1)
    
    email = sys.argv[1]
    password = sys.argv[2]
    
    # Run all tests
    run_all_tests(email, password)

if __name__ == "__main__":
    main() 