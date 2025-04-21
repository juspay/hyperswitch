import json
import traceback
import sys
from typing import Dict, Any
import requests

# Import our new logging utilities
from .utils import Logger, ApiDebugger, timed_execution

def signin(email: str, password: str) -> Dict[str, Any]:
    """
    Sign in to Hyperswitch with email and password and get an authentication token.
    
    Args:
        email: User's email address
        password: User's password
        
    Returns:
        A dictionary containing the token, user_id and other user details.
        Example success: {'token': 'totp_...', 'user_id': '...', ...}
        Example error: {'error': 'Invalid credentials'}
    """
    try:
        # Log attempt with sensitive data masked
        Logger.info(f"Attempting to sign in user: {email}", {"email_length": len(email)})
        
        # Define API endpoint and request body
        url = "http://localhost:8080/user/signin"
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
        payload = {
        "email": email,
        "password": password
    }
        
        # Log the API request (with password masked)
        masked_payload = {**payload, "password": "[MASKED]"}
        ApiDebugger.log_request("POST", url, headers, masked_payload)
        
        # Make the API call
        response = requests.post(url, headers=headers, json=payload)
        
        # Log the response (with sensitive data handled by ApiDebugger)
        ApiDebugger.log_response(
            response.status_code, 
            dict(response.headers), 
            response.content
        )
        
        # Process the response
        if response.status_code == 200:
            try:
                data = response.json()
                Logger.info("Successful sign-in", {"user_id": data.get("user_id", "unknown")})
                return data
            except ValueError:
                error_msg = "API returned success but with invalid JSON"
                Logger.error(error_msg, {"response_text": response.text[:100]})
                return {"error": error_msg}
        else:
            # Handle error response
            error_details = {}
            try:
                error_details = response.json()
            except ValueError:
                error_details = {"response_text": response.text[:100]}
            
            error_msg = f"Sign-in failed with status {response.status_code}"
            Logger.error(error_msg, {"status": response.status_code, "details": error_details})
            return {
                "error": error_msg,
                "status": response.status_code,
                "details": error_details
            }
            
    except requests.RequestException as e:
        # Handle network-related errors
        error_msg = f"Request failed: {str(e)}"
        Logger.exception("Network error during sign-in", e)
        return {"error": error_msg}
    except Exception as e:
        # Handle any other unexpected errors
        error_msg = f"An unexpected error occurred: {str(e)}"
        Logger.exception("Unexpected error during sign-in", e)
        return {"error": error_msg, "traceback": traceback.format_exc()}

def signout(jwt_token: str) -> Dict[str, Any]:
    """
    Sign out a user by invalidating their JWT token.
    
    Args:
        jwt_token: The JWT token to invalidate
        
    Returns:
        A dictionary containing the status of the sign-out operation.
        Example success: {'status': 'success', 'message': 'Successfully signed out'}
        Example error: {'error': 'Invalid token', 'status': 401}
    """
    try:
        # Log the sign-out attempt
        Logger.info("Attempting to sign out user")
        
        # Define API endpoint
        url = "http://localhost:8080/user/signout"
        headers = {
            "Authorization": f"Bearer {jwt_token}",
            "Accept": "application/json"
        }
        
        # Log the API request (without exposing full token)
        masked_headers = {**headers}
        if "Authorization" in masked_headers:
            token_prefix = jwt_token[:10] if len(jwt_token) > 10 else jwt_token
            masked_headers["Authorization"] = f"Bearer {token_prefix}..."
        ApiDebugger.log_request("POST", url, masked_headers)
        
        # Make the API call
        response = requests.post(url, headers=headers)
        
        # Log the response
        ApiDebugger.log_response(
            response.status_code, 
            dict(response.headers), 
            response.content
        )
        
        # Process the response
        if response.status_code == 200:
            try:
                # Attempt to parse JSON, default to empty dict if body is empty or invalid
                try:
                    data = response.json()
                    if not isinstance(data, dict): # Ensure it's a dictionary
                        data = {}
                except ValueError: # Handle cases where body is not valid JSON
                    data = {}
                    
                Logger.info("Successfully signed out user")
                # Merge the base success message with data if it's not empty
                result = {
                    "status": "success",
                    "message": "Successfully signed out"
                }
                if data: # Only merge if data is not empty
                    result.update(data)
                return result
            except ValueError: # This outer except seems redundant now, but kept for safety
                # If somehow still an issue, return basic success
                Logger.warning("Sign out successful, but response body processing issue occurred.")
                return {
                    "status": "success",
                    "message": "Successfully signed out (response body issue)"
                }
        else:
            # Handle error response
            error_details = {}
            try:
                error_details = response.json()
            except ValueError:
                error_details = {"response_text": response.text[:100]}
            
            error_msg = f"Sign-out failed with status {response.status_code}"
            Logger.error(error_msg, {"status": response.status_code, "details": error_details})
            return {
                "error": error_msg,
                "status": response.status_code,
                "details": error_details
            }
            
    except requests.RequestException as e:
        # Handle network-related errors
        error_msg = f"Request failed: {str(e)}"
        Logger.exception("Network error during sign-out", e)
        return {"error": error_msg}
    except Exception as e:
        # Handle any other unexpected errors
        error_msg = f"An unexpected error occurred: {str(e)}"
        Logger.exception("Unexpected error during sign-out", e)
        return {"error": error_msg, "traceback": traceback.format_exc()}

def terminate_2fa(totp_token: str, skip_two_factor_auth: bool = True) -> Dict[str, Any]:
    """
    Terminates the 2FA check using the token from signin and retrieves the user info JWT.
    This is a simplified implementation directly using requests, as an alternative to the
    more complex implementation in server.py that uses the SDK.

    Args:
        totp_token: The authentication token obtained from the signin function.
        skip_two_factor_auth: Whether to skip the actual 2FA check (defaults to True).

    Returns:
        A dictionary containing the user info JWT or an error.
        Example success: {'user_info_token': 'eyJh...', 'user_id': '...', ...}
        Example error: {'error': 'API call failed...'}
    """
    try:
        # Log the 2FA termination attempt
        Logger.info(f"Attempting to terminate 2FA with skip_two_factor_auth={skip_two_factor_auth}")
        
        # Define API endpoint
        url = f"http://localhost:8080/user/2fa/terminate"
        if skip_two_factor_auth:
            url += "?skip_two_factor_auth=true"
            
        headers = {
            "Authorization": f"Bearer {totp_token}",
            "Accept": "application/json"
        }
        
        # Log the API request (without exposing full token)
        masked_headers = {**headers}
        if "Authorization" in masked_headers:
            token_prefix = totp_token[:10] if len(totp_token) > 10 else totp_token
            masked_headers["Authorization"] = f"Bearer {token_prefix}..."
        ApiDebugger.log_request("GET", url, masked_headers)
        
        # Make the API call
        response = requests.get(url, headers=headers)
        
        # Log the response
        ApiDebugger.log_response(
            response.status_code, 
            dict(response.headers), 
            response.content
        )
        
        # Process the response
        if response.status_code == 200:
            try:
                data = response.json()
                if "token" in data:
                    # Rename token for clarity
                    data["user_info_token"] = data.pop("token")
                Logger.info("Successfully terminated 2FA")
                return data
            except ValueError:
                error_msg = "API returned success but with invalid JSON"
                Logger.error(error_msg, {"response_text": response.text[:100]})
                return {"error": error_msg}
        else:
            # Handle error response
            error_details = {}
            try:
                error_details = response.json()
            except ValueError:
                error_details = {"response_text": response.text[:100]}
            
            error_msg = f"2FA termination failed with status {response.status_code}"
            Logger.error(error_msg, {"status": response.status_code, "details": error_details})
            return {
                "error": error_msg,
                "status": response.status_code,
                "details": error_details
            }
            
    except requests.RequestException as e:
        # Handle network-related errors
        error_msg = f"Request failed: {str(e)}"
        Logger.exception("Network error during 2FA termination", e)
        return {"error": error_msg}
    except Exception as e:
        # Handle any other unexpected errors
        error_msg = f"An unexpected error occurred: {str(e)}"
        Logger.exception("Unexpected error during 2FA termination", e)
        return {"error": error_msg, "traceback": traceback.format_exc()}