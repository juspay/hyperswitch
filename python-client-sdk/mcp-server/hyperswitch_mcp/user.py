import json
import traceback
from typing import Dict, Any
import requests

# Import our logging utilities
from .utils import Logger, ApiDebugger, timed_execution

def get_user_info(jwt_token: str) -> Dict[str, Any]:
    """
    Retrieve user information using the JWT token.
    
    Args:
        jwt_token: The JWT token obtained from the 2FA termination
        
    Returns:
        A dictionary containing the user information or an error.
        Example success: {
            'user_id': 'user_12345', 
            'email': 'user@example.com',
            'name': 'John Doe',
            'created_at': '2023-01-01T12:00:00Z',
            'roles': ['admin'],
            'status': 'active'
        }
        Example error: {'error': 'Failed to get user info', 'status': 401}
    """
    try:
        # Log the attempt
        Logger.info("Attempting to get user information")
        
        # Define API endpoint
        url = "http://localhost:8080/user"
        headers = {
            "Authorization": f"Bearer {jwt_token}",
            "Accept": "application/json"
        }
        
        # Log the API request (without exposing full token)
        masked_headers = {**headers}
        if "Authorization" in masked_headers:
            token_prefix = jwt_token[:10] if len(jwt_token) > 10 else jwt_token
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
                Logger.info("Successfully retrieved user info", {"user_id": data.get("user_id", "unknown")})
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
            
            error_msg = f"Failed to get user info with status {response.status_code}"
            Logger.error(error_msg, {"status": response.status_code, "details": error_details})
            return {
                "error": error_msg,
                "status": response.status_code,
                "details": error_details
            }
            
    except requests.RequestException as e:
        # Handle network-related errors
        error_msg = f"Request failed: {str(e)}"
        Logger.exception("Network error during get user info", e)
        return {"error": error_msg}
    except Exception as e:
        # Handle any other unexpected errors
        error_msg = f"An unexpected error occurred: {str(e)}"
        Logger.exception("Unexpected error during get user info", e)
        return {"error": error_msg, "traceback": traceback.format_exc()}

def update_user(jwt_token: str, name: str = None, phone: str = None) -> Dict[str, Any]:
    """
    Update the user's profile information.
    
    Args:
        jwt_token: The JWT token obtained from the 2FA termination
        name: New name for the user (optional)
        phone: New phone number for the user (optional)
        
    Returns:
        A dictionary containing the updated user information or an error.
        Example success: {
            'user_id': 'user_12345', 
            'email': 'user@example.com',
            'name': 'John Doe',
            'phone': '+1234567890',
            'updated_at': '2023-01-01T12:00:00Z'
        }
        Example error: {'error': 'Failed to update user', 'status': 400}
    """
    try:
        # Log the attempt
        Logger.info("Attempting to update user information")
        
        # Define API endpoint
        url = "http://localhost:8080/user/update"
        headers = {
            "Authorization": f"Bearer {jwt_token}",
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
        
        # Build the payload with only provided fields
        payload = {}
        if name is not None:
            payload["name"] = name
        if phone is not None:
            payload["phone"] = phone
            
        # Check if payload is empty
        if not payload:
            error_msg = "No update fields provided"
            Logger.error(error_msg)
            return {"error": error_msg}
        
        # Log the API request (without exposing full token)
        masked_headers = {**headers}
        if "Authorization" in masked_headers:
            token_prefix = jwt_token[:10] if len(jwt_token) > 10 else jwt_token
            masked_headers["Authorization"] = f"Bearer {token_prefix}..."
        ApiDebugger.log_request("POST", url, masked_headers, payload)
        
        # Make the API call using POST instead of PUT
        response = requests.post(url, headers=headers, json=payload)
        
        # Log the response
        ApiDebugger.log_response(
            response.status_code, 
            dict(response.headers), 
            response.content
        )
        
        # Process the response
        if response.status_code == 200:
            # Check if response body is empty before trying to parse
            if response.text:
                try:
                    data = response.json()
                    Logger.info("Successfully updated user", {"user_id": data.get("user_id", "unknown")})
                    return data
                except ValueError:
                    # Status is 200, but JSON is invalid (unexpected)
                    error_msg = "API returned success but with invalid JSON"
                    Logger.error(error_msg, {"response_text": response.text[:100]})
                    return {"error": error_msg}
            else:
                # Status is 200, but body is empty (consider this success)
                Logger.info("Successfully updated user (API returned empty body)")
                return {"status": "success", "message": "User updated successfully (empty response)"}
        else:
            # Handle error response (status code is not 200)
            error_details = {}
            try:
                error_details = response.json()
            except ValueError:
                error_details = {"response_text": response.text[:100]}
            
            error_msg = f"Failed to update user with status {response.status_code}"
            Logger.error(error_msg, {"status": response.status_code, "details": error_details})
            return {
                "error": error_msg,
                "status": response.status_code,
                "details": error_details
            }
            
    except requests.RequestException as e:
        # Handle network-related errors
        error_msg = f"Request failed: {str(e)}"
        Logger.exception("Network error during update user", e)
        return {"error": error_msg}
    except Exception as e:
        # Handle any other unexpected errors
        error_msg = f"An unexpected error occurred: {str(e)}"
        Logger.exception("Unexpected error during update user", e)
        return {"error": error_msg, "traceback": traceback.format_exc()}

def change_password(jwt_token: str, current_password: str, new_password: str) -> Dict[str, Any]:
    """
    Change the user's password.

    Args:
        jwt_token: The JWT token obtained from the 2FA termination.
        current_password: The user's current password.
        new_password: The desired new password.

    Returns:
        A dictionary indicating success or failure.
        Example success: {'status': 'success', 'message': 'Password changed successfully'}
        Example error: {'error': 'Incorrect current password', 'status': 401}
    """
    try:
        # Log the attempt
        Logger.info("Attempting to change password")
        
        # Define API endpoint
        url = "http://localhost:8080/user/change_password"
        headers = {
            "Authorization": f"Bearer {jwt_token}",
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
        payload = {
            "old_password": current_password,
            "new_password": new_password
        }
        
        # Log the API request (masking passwords)
        masked_headers = {**headers}
        if "Authorization" in masked_headers:
            token_prefix = jwt_token[:10] if len(jwt_token) > 10 else jwt_token
            masked_headers["Authorization"] = f"Bearer {token_prefix}..."
        masked_payload = {
            "old_password": "[MASKED]",
            "new_password": "[MASKED]"
        }
        ApiDebugger.log_request("POST", url, masked_headers, masked_payload)
        
        # Make the API call
        response = requests.post(url, headers=headers, json=payload)
        
        # Log the response
        ApiDebugger.log_response(
            response.status_code, 
            dict(response.headers), 
            response.content
        )
        
        # Process the response
        if response.status_code == 200:
            # Assume success on 200 OK, check body for details if available
            try:
                data = response.json()
                message = data.get("message", "Password changed successfully")
            except ValueError:
                message = "Password changed successfully (no response body)"
            
            Logger.info(message)
            return {"status": "success", "message": message}
        else:
            # Handle error response
            error_details = {}
            try:
                error_details = response.json()
                error_message = error_details.get("message", "Unknown error")
            except ValueError:
                error_details = {"response_text": response.text[:100]}
                error_message = response.text[:100] or "Unknown error"
            
            error_log_msg = f"Failed to change password with status {response.status_code}"
            Logger.error(error_log_msg, {"status": response.status_code, "details": error_details})
            return {
                "error": error_message,
                "status": response.status_code,
                "details": error_details
            }
            
    except requests.RequestException as e:
        # Handle network-related errors
        error_msg = f"Request failed: {str(e)}"
        Logger.exception("Network error during change password", e)
        return {"error": error_msg}
    except Exception as e:
        # Handle any other unexpected errors
        error_msg = f"An unexpected error occurred: {str(e)}"
        Logger.exception("Unexpected error during change password", e)
        return {"error": error_msg, "traceback": traceback.format_exc()}

def initiate_password_reset(email: str) -> Dict[str, Any]:
    """
    Initiates the password reset process for the given email.
    The API is expected to send a reset token to the user's email.

    Args:
        email: The email address of the user.

    Returns:
        A dictionary indicating success or failure.
        Example success: {'status': 'success', 'message': 'Password reset email sent'}
        Example error: {'error': 'User not found', 'status': 404}
    """
    try:
        # Log the attempt
        Logger.info(f"Attempting to initiate password reset for email: {email}")
        
        # Define API endpoint
        url = "http://localhost:8080/user/reset_password"
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
        payload = {
            "email": email,
            "token": ""
        }
        
        # Log the API request
        ApiDebugger.log_request("POST", url, headers, payload)
        
        # Send POST with the modified JSON payload
        response = requests.post(url, headers=headers, json=payload)
        
        # Log the response
        ApiDebugger.log_response(
            response.status_code, 
            dict(response.headers), 
            response.content
        )
        
        # Process the response
        if response.status_code == 200:
            # Assume success on 200 OK
            try:
                data = response.json()
                message = data.get("message", "Password reset initiated successfully. Check email.")
            except ValueError:
                message = "Password reset initiated successfully (no response body). Check email."
            
            Logger.info(message)
            return {"status": "success", "message": message}
        else:
            # Handle error response
            error_details = {}
            try:
                error_details = response.json()
                error_message = error_details.get("message", "Unknown error")
            except ValueError:
                error_details = {"response_text": response.text[:100]}
                error_message = response.text[:100] or "Unknown error"
            
            error_log_msg = f"Failed to initiate password reset with status {response.status_code}"
            Logger.error(error_log_msg, {"status": response.status_code, "details": error_details})
            return {
                "error": error_message,
                "status": response.status_code,
                "details": error_details
            }
            
    except requests.RequestException as e:
        # Handle network-related errors
        error_msg = f"Request failed: {str(e)}"
        Logger.exception("Network error during password reset initiation", e)
        return {"error": error_msg}
    except Exception as e:
        # Handle any other unexpected errors
        error_msg = f"An unexpected error occurred: {str(e)}"
        Logger.exception("Unexpected error during password reset initiation", e)
        return {"error": error_msg, "traceback": traceback.format_exc()}

def reset_password_confirm(reset_token: str, new_password: str) -> Dict[str, Any]:
    """
    Confirms the password reset using the provided token and sets the new password.

    Args:
        reset_token: The password reset token received by the user.
        new_password: The desired new password.

    Returns:
        A dictionary indicating success or failure.
        Example success: {'status': 'success', 'message': 'Password reset successfully'}
        Example error: {'error': 'Invalid or expired token', 'status': 400}
    """
    try:
        # Log the attempt
        Logger.info("Attempting to confirm password reset")
        
        # Define API endpoint (likely same as initiation)
        url = "http://localhost:8080/user/reset_password"
        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
        payload = {
            "token": reset_token,
            "new_password": new_password
        }
        
        # Log the API request (masking password and potentially token)
        masked_token = reset_token[:4] + "..." if len(reset_token) > 8 else reset_token
        masked_payload = {
            "token": masked_token,
            "new_password": "[MASKED]"
        }
        ApiDebugger.log_request("POST", url, headers, masked_payload)
        
        # Make the API call
        response = requests.post(url, headers=headers, json=payload)
        
        # Log the response
        ApiDebugger.log_response(
            response.status_code, 
            dict(response.headers), 
            response.content
        )
        
        # Process the response
        if response.status_code == 200:
            # Assume success on 200 OK
            try:
                data = response.json()
                message = data.get("message", "Password reset successfully.")
            except ValueError:
                message = "Password reset successfully (no response body)."
            
            Logger.info(message)
            return {"status": "success", "message": message}
        else:
            # Handle error response
            error_details = {}
            try:
                error_details = response.json()
                error_message = error_details.get("message", "Unknown error")
            except ValueError:
                error_details = {"response_text": response.text[:100]}
                error_message = response.text[:100] or "Unknown error"
            
            error_log_msg = f"Failed to confirm password reset with status {response.status_code}"
            Logger.error(error_log_msg, {"status": response.status_code, "details": error_details})
            return {
                "error": error_message,
                "status": response.status_code,
                "details": error_details
            }
            
    except requests.RequestException as e:
        # Handle network-related errors
        error_msg = f"Request failed: {str(e)}"
        Logger.exception("Network error during password reset confirmation", e)
        return {"error": error_msg}
    except Exception as e:
        # Handle any other unexpected errors
        error_msg = f"An unexpected error occurred: {str(e)}"
        Logger.exception("Unexpected error during password reset confirmation", e)
        return {"error": error_msg, "traceback": traceback.format_exc()}

def verify_email(jwt_token: str, verification_token: str) -> Dict[str, Any]:
    """
    Verifies the user's email address using the provided token.

    Args:
        jwt_token: The user's JWT token for authorization.
        verification_token: The email verification token sent to the user.

    Returns:
        A dictionary indicating success or failure.
        Example success: {'status': 'success', 'message': 'Email verified successfully'}
        Example error: {'error': 'Invalid or expired token', 'status': 400}
    """
    try:
        # Log the attempt
        Logger.info("Attempting to verify email")
        
        # Define API endpoint
        url = "http://localhost:8080/user/verify_email"
        headers = {
            "Authorization": f"Bearer {jwt_token}",
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
        payload = {
            "token": verification_token
        }
        
        # Log the API request (masking token partially)
        masked_headers = {**headers}
        if "Authorization" in masked_headers:
            token_prefix = jwt_token[:10] if len(jwt_token) > 10 else jwt_token
            masked_headers["Authorization"] = f"Bearer {token_prefix}..."
            
        masked_token = verification_token[:4] + "..." if len(verification_token) > 8 else verification_token
        masked_payload = {"token": masked_token}
        ApiDebugger.log_request("POST", url, masked_headers, masked_payload)
        
        # Make the API call
        response = requests.post(url, headers=headers, json=payload)
        
        # Log the response
        ApiDebugger.log_response(
            response.status_code, 
            dict(response.headers), 
            response.content
        )
        
        # Process the response
        if response.status_code == 200:
            # Assume success on 200 OK
            try:
                data = response.json()
                message = data.get("message", "Email verified successfully.")
            except ValueError:
                message = "Email verified successfully (no response body)."
            
            Logger.info(message)
            return {"status": "success", "message": message}
        else:
            # Handle error response
            error_details = {}
            try:
                error_details = response.json()
                error_message = error_details.get("message", "Unknown error")
            except ValueError:
                error_details = {"response_text": response.text[:100]}
                error_message = response.text[:100] or "Unknown error"
            
            error_log_msg = f"Failed to verify email with status {response.status_code}"
            Logger.error(error_log_msg, {"status": response.status_code, "details": error_details})
            return {
                "error": error_message,
                "status": response.status_code,
                "details": error_details
            }
            
    except requests.RequestException as e:
        # Handle network-related errors
        error_msg = f"Request failed: {str(e)}"
        Logger.exception("Network error during email verification", e)
        return {"error": error_msg}
    except Exception as e:
        # Handle any other unexpected errors
        error_msg = f"An unexpected error occurred: {str(e)}"
        Logger.exception("Unexpected error during email verification", e)
        return {"error": error_msg, "traceback": traceback.format_exc()} 