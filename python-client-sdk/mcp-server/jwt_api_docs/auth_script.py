#!/usr/bin/env python3
"""
Authentication Script for Hyperswitch

This script demonstrates how to sign in and terminate 2FA to get a JWT token
that can be used for testing the Business Profiles API.

Usage:
    python auth_script.py --email YOUR_EMAIL --password YOUR_PASSWORD
    python auth_script.py --email YOUR_EMAIL --password YOUR_PASSWORD --token-only
    python auth_script.py --email YOUR_EMAIL --password YOUR_PASSWORD --json
"""

import argparse
import json
import requests
import logging
import sys
import time
from urllib.parse import urljoin

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    handlers=[
        logging.FileHandler("jwt_api_docs/auth_script.log"),
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger(__name__)

# Base URL - strictly localhost:8080 only
BASE_URL = "http://localhost:8080"

def sign_in(email, password):
    """Sign in to Hyperswitch and get a TOTP token."""
    url = urljoin(BASE_URL, "/user/signin")
    
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json"
    }
    
    payload = {
        "email": email,
        "password": password
    }
    
    logger.info(f"Attempting to sign in user: {email}")
    
    try:
        response = requests.post(url, headers=headers, json=payload, timeout=10)
        
        if response.status_code == 200:
            result = response.json()
            if "token" in result:
                logger.info("Sign in successful, TOTP token obtained")
                return {"success": True, "totp_token": result["token"]}
            else:
                logger.error("Sign in response missing token field")
                return {"success": False, "error": "Response missing token field"}
        else:
            try:
                error_data = response.json()
                error_message = error_data.get("message", "Unknown error")
            except:
                error_message = response.text or "Unknown error"
                
            logger.error(f"Sign in failed: {error_message}")
            return {"success": False, "error": error_message, "status_code": response.status_code}
            
    except requests.RequestException as e:
        logger.error(f"Request exception during sign in: {str(e)}")
        return {"success": False, "error": str(e)}
    except Exception as e:
        logger.exception(f"Unexpected error during sign in: {str(e)}")
        return {"success": False, "error": f"Unexpected error: {str(e)}"}

def terminate_2fa(totp_token, skip_two_factor_auth=True):
    """Terminate 2FA and get a JWT token."""
    url = urljoin(BASE_URL, f"/user/2fa/terminate?skip_two_factor_auth={str(skip_two_factor_auth).lower()}")
    
    headers = {
        "Authorization": f"Bearer {totp_token}",
        "Accept": "application/json"
    }
    
    logger.info(f"Attempting to terminate 2FA with skip_two_factor_auth={skip_two_factor_auth}")
    
    try:
        response = requests.get(url, headers=headers, timeout=10)
        
        if response.status_code == 200:
            result = response.json()
            if "token" in result:
                logger.info("2FA terminated successfully, JWT token obtained")
                return {"success": True, "jwt_token": result["token"]}
            else:
                logger.error("2FA termination response missing token field")
                return {"success": False, "error": "Response missing token field"}
        else:
            try:
                error_data = response.json()
                error_message = error_data.get("message", "Unknown error")
            except:
                error_message = response.text or "Unknown error"
                
            logger.error(f"2FA termination failed: {error_message}")
            return {"success": False, "error": error_message, "status_code": response.status_code}
            
    except requests.RequestException as e:
        logger.error(f"Request exception during 2FA termination: {str(e)}")
        return {"success": False, "error": str(e)}
    except Exception as e:
        logger.exception(f"Unexpected error during 2FA termination: {str(e)}")
        return {"success": False, "error": f"Unexpected error: {str(e)}"}

def authenticate(email, password):
    """Complete authentication flow: sign in and terminate 2FA."""
    # Step 1: Sign in
    sign_in_result = sign_in(email, password)
    
    if not sign_in_result["success"]:
        return sign_in_result
    
    # Step 2: Terminate 2FA
    totp_token = sign_in_result["totp_token"]
    terminate_result = terminate_2fa(totp_token)
    
    if not terminate_result["success"]:
        return terminate_result
    
    # Success! Return both tokens and success status
    return {
        "success": True,
        "totp_token": totp_token,
        "jwt_token": terminate_result["jwt_token"]
    }

def main():
    """Main function to run the authentication flow."""
    parser = argparse.ArgumentParser(description="Authenticate with Hyperswitch")
    parser.add_argument("--email", required=True, help="Email address for sign in")
    parser.add_argument("--password", required=True, help="Password for sign in")
    parser.add_argument("--json", action="store_true", help="Output as JSON for scripting")
    parser.add_argument("--token-only", action="store_true", help="Output only the JWT token (for shell script use)")
    
    args = parser.parse_args()
    
    # If token-only is specified, completely disable logging
    if args.token_only:
        logging.disable(logging.CRITICAL)  # This disables all logging at CRITICAL level and below
    
    # Run the authentication flow
    result = authenticate(args.email, args.password)
    
    # Output only the token if requested (for shell scripts)
    if args.token_only:
        if result["success"]:
            print(result["jwt_token"])
            return 0
        else:
            return 1
    
    # Output as JSON if requested (for scripting)
    if args.json:
        print(json.dumps(result))
        return 0 if result["success"] else 1
    
    # Otherwise print formatted output for human readability
    if result["success"]:
        print("\n" + "=" * 60)
        print(" AUTHENTICATION SUCCESSFUL ".center(60))
        print("=" * 60)
        print(f"\nJWT Token: {result['jwt_token'][:15]}...{result['jwt_token'][-15:]}")
        print("\nUse this JWT token for testing the Business Profiles API:")
        print(f"\npython jwt_api_docs/group4_business_profiles/test_business_profiles.py \\")
        print(f"  --token {result['jwt_token']} \\")
        print(f"  --api_key YOUR_API_KEY \\")
        print(f"  --account_id YOUR_ACCOUNT_ID \\")
        print(f"  --test all")
        print("\n" + "=" * 60)
    else:
        print("\n" + "=" * 60)
        print(" AUTHENTICATION FAILED ".center(60))
        print("=" * 60)
        print(f"\nError: {result['error']}")
        print(f"Status code: {result.get('status_code', 'N/A')}")
        print("\nPlease check your credentials and try again.")
        print("\n" + "=" * 60)
    
    return 0 if result["success"] else 1

if __name__ == "__main__":
    sys.exit(main()) 