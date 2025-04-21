#!/usr/bin/env python3
"""
Direct Test Script for Hyperswitch MCP Tools

This script tests the MCP tools directly for user authentication flow in Hyperswitch.
"""

import sys
import json
import time
import argparse
import base64

# Set PYTHONPATH to include the MCP server
sys.path.append('/home/jarnura/github/hyperswitch/python-client-sdk/mcp-server')

# Import the MCP tools directly
try:
    from hyperswitch_mcp.server import (
        say_hello,
        signin_tool, 
        terminate_2fa_tool,
        get_user_info_tool,
        update_user_profile_tool,
        signout_tool
    )
    print("✅ Successfully imported MCP tools")
except ImportError as e:
    print(f"❌ Failed to import MCP tools: {e}")
    sys.exit(1)

def print_separator():
    """Print a separator for better readability"""
    print("\n" + "=" * 50 + "\n")

def decode_jwt(token):
    """Decode JWT token parts and print them"""
    try:
        parts = token.split('.')
        if len(parts) != 3:
            return "Invalid JWT format"
            
        # Decode header
        padded_header = parts[0] + '=' * (4 - len(parts[0]) % 4)
        header_bytes = base64.urlsafe_b64decode(padded_header)
        header = json.loads(header_bytes)
        
        # Decode payload
        padded_payload = parts[1] + '=' * (4 - len(parts[1]) % 4)
        payload_bytes = base64.urlsafe_b64decode(padded_payload)
        payload = json.loads(payload_bytes)
        
        return {
            "header": header,
            "payload": payload
        }
    except Exception as e:
        return f"Error decoding token: {str(e)}"

def test_say_hello():
    """Test the say_hello function"""
    print("Testing say_hello...")
    result = say_hello("Test User")
    print(f"Result: {json.dumps(result, indent=2)}")
    print("✅ say_hello test completed")

def test_auth_flow(email, password):
    """Test the complete authentication flow"""
    print(f"Testing authentication flow for {email}...")
    
    # Step 1: Sign in
    print("\nStep 1: Sign in")
    signin_result = signin_tool(email, password)
    print(f"Sign in result: {json.dumps(signin_result, indent=2)}")
    
    if "error" in signin_result:
        print(f"❌ Sign in failed: {signin_result.get('error')}")
        return False
    
    totp_token = signin_result.get("totp_token")
    if not totp_token:
        print("❌ No TOTP token received")
        return False
        
    # Decode and print TOTP token
    print("\nTOTP Token Analysis:")
    totp_decoded = decode_jwt(totp_token)
    print(f"Decoded TOTP token: {json.dumps(totp_decoded, indent=2)}")
    
    # Step 2: Terminate 2FA
    print("\nStep 2: Terminate 2FA")
    terminate_result = terminate_2fa_tool(totp_token, "True")
    print(f"2FA result: {json.dumps(terminate_result, indent=2)}")
    
    if "error" in terminate_result:
        print(f"❌ 2FA termination failed: {terminate_result.get('error')}")
        return False
    
    jwt_token = terminate_result.get("user_info_token")
    if not jwt_token:
        print("❌ No JWT token received")
        return False
        
    # Decode and print JWT token
    print("\nJWT Token Analysis:")
    jwt_decoded = decode_jwt(jwt_token)
    print(f"Decoded JWT token: {json.dumps(jwt_decoded, indent=2)}")
    
    # Step 3: Get user info
    print("\nStep 3: Get user info")
    userinfo_result = get_user_info_tool(jwt_token)
    print(f"User info result: {json.dumps(userinfo_result, indent=2)}")
    
    if "error" in userinfo_result:
        print(f"❌ Get user info failed: {userinfo_result.get('error')}")
        return False
    
    # Step 4: Update profile
    print("\nStep 4: Update profile")
    test_name = f"Updated User {int(time.time())}"
    update_result = update_user_profile_tool(jwt_token, test_name)
    print(f"Update result: {json.dumps(update_result, indent=2)}")
    
    if "error" in update_result:
        print(f"❌ Update profile failed: {update_result.get('error')}")
        return False
    
    # Step 5: Sign out
    print("\nStep 5: Sign out")
    signout_result = signout_tool(jwt_token)
    print(f"Sign out result: {json.dumps(signout_result, indent=2)}")
    
    if "error" in signout_result:
        print(f"❌ Sign out failed: {signout_result.get('error')}")
        return False
    
    print("\n✅ Authentication flow test completed successfully")
    return True

def main():
    """Main function"""
    # Parse command line arguments
    parser = argparse.ArgumentParser(description="Test Hyperswitch MCP Tools")
    parser.add_argument("--email", default="test@example.com", help="Email for authentication")
    parser.add_argument("--password", default="testpassword", help="Password for authentication")
    parser.add_argument("--skip-hello", action="store_true", help="Skip the hello test")
    args = parser.parse_args()
    
    print_separator()
    print("HYPERSWITCH MCP TOOLS DIRECT TEST")
    print_separator()
    
    # Test say_hello
    if not args.skip_hello:
        test_say_hello()
        print_separator()
    
    # Test auth flow with provided credentials
    success = test_auth_flow(args.email, args.password)
    print_separator()
    
    if success:
        print("✅ All tests completed successfully!")
        return 0
    else:
        print("❌ Some tests failed. See details above.")
        return 1

if __name__ == "__main__":
    sys.exit(main()) 