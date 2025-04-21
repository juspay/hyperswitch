from typing import Dict, Any
from mcp.server.fastmcp import FastMCP
import json
import traceback
import sys
import time

# Import our new logging utilities
from .utils import Logger, ApiDebugger, LogLevel, initialize_logging, timed_execution

# Import directly from modules
# Removed imports for non-existent modules:
# from .api_key import create_api_key, retrieve_api_key, update_api_key 
# from .merchant import create_merchant_account, retrieve_merchant_account, update_merchant_account
# from .profile import create_profile, retrieve_profile, update_profile, list_profiles

# Keep necessary imports
from .auth import signin, terminate_2fa, signout
from .user import (
    get_user_info, 
    update_user,
    change_password,
    initiate_password_reset,
    reset_password_confirm,
    verify_email
)
from .business_profiles import (
    list_business_profiles,
    get_business_profile,
    create_business_profile,
    update_business_profile,
    delete_business_profile
)
from hyperswitch.api_client import ApiClient, ApiException
from hyperswitch.configuration import Configuration
# from hyperswitch.api.user_api import UserApi # UserApi might not exist or be needed for direct call
# from hyperswitch.api.profile_api import ProfileApi # ProfileApi likely not needed for direct requests

# Initialize the MCP server with proper JSON configuration
mcp_config = {
    "name": "hyperswitch-mcp",
    "version": "1.0.0",
    "description": "Hyperswitch MCP Server",
    "tools": [] # Tools will be added dynamically by @mcp.tool
}

# Revert to simpler initialization, like the working server
mcp = FastMCP(mcp_config["name"]) # Use name directly from config

# --- Existing/Added Tools Start ---

@mcp.tool("Say hello to someone")
def say_hello(name: str) -> Dict[str, Any]:
    """
    A simple hello world tool that greets the given name.
    
    Args:
        name: The name to greet
        
    Returns:
        A dictionary containing the greeting message
    """
    Logger.info(f"Saying hello to {name}")
    return {"message": f"Hello, {name}!"}

@mcp.tool("Sign in to Hyperswitch")
def signin_tool(email: str, password: str) -> Dict[str, Any]:
    """
    Sign in to Hyperswitch and get an authentication token.
    
    Args:
        email: User's email address
        password: User's password
        
    Returns:
        A dictionary containing the authentication token (totp_token) and user details.
        Example success: {'totp_token': 'totp_...', 'user_id': '...', ...}
        Example error: {'error': 'Invalid credentials'}
    """
    # Mask the password in logs
    masked_email = email
    Logger.info(f"Attempting to sign in user: {masked_email}", {"email_provided": True})
    
    # The signin function might return a dict with a 'token' key
    result = signin(email, password)
    
    if "token" in result and "error" not in result: # Check for success before renaming
        # Rename token for clarity in the sequence
        result["totp_token"] = result.pop("token")
        Logger.info("Sign in successful, TOTP token obtained")
    else:
        Logger.error("Sign in failed", {"error": result.get("error", "Unknown error")})
        
    return result

@mcp.tool("Sign out from Hyperswitch")
def signout_tool(jwt_token: str) -> Dict[str, Any]:
    """
    Signs out a user by invalidating their JWT token.
    
    Args:
        jwt_token: The JWT token to invalidate (obtained from terminate_2fa_tool)
        
    Returns:
        A dictionary containing the status of the sign-out operation.
        Example success: {'status': 'success', 'message': 'Successfully signed out'}
        Example error: {'error': 'Invalid token', 'status': 401}
    """
    # Log the sign-out attempt (without exposing the token)
    token_prefix = jwt_token[:10] if len(jwt_token) > 10 else jwt_token
    Logger.info(f"Attempting to sign out user with token prefix: {token_prefix}...")
    
    # Call the signout function
    result = signout(jwt_token)
    
    if "error" not in result:
        Logger.info("Sign out successful")
    else:
        Logger.error("Sign out failed", {"error": result.get("error", "Unknown error")})
    
    return result

@mcp.tool("Get User Info")
def get_user_info_tool(jwt_token: str) -> Dict[str, Any]:
    """
    Retrieves information about the current user using their JWT token.
    
    Args:
        jwt_token: The JWT token obtained from the terminate_2fa_tool
        
    Returns:
        A dictionary containing the user information.
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
    # Log the attempt (without exposing the token)
    token_prefix = jwt_token[:10] if len(jwt_token) > 10 else jwt_token
    Logger.info(f"Attempting to get user info with token prefix: {token_prefix}...")
    
    # Call the get_user_info function
    result = get_user_info(jwt_token)
    
    if "error" not in result:
        Logger.info("Successfully retrieved user info", {"user_id": result.get("user_id", "unknown")})
    else:
        Logger.error("Failed to get user info", {"error": result.get("error", "Unknown error")})
    
    return result

@mcp.tool("Update User Profile")
def update_user_profile_tool(jwt_token: str, name: str = None, phone: str = None) -> Dict[str, Any]:
    """
    Updates the user's profile information.
    
    Args:
        jwt_token: The JWT token obtained from the terminate_2fa_tool
        name: New name for the user (optional)
        phone: New phone number for the user (optional)
        
    Returns:
        A dictionary containing the updated user information.
        Example success: {
            'user_id': 'user_12345', 
            'email': 'user@example.com',
            'name': 'John Doe',
            'phone': '+1234567890',
            'updated_at': '2023-01-01T12:00:00Z'
        }
        Example error: {'error': 'Failed to update user', 'status': 400}
    """
    # Log the attempt (without exposing the token)
    token_prefix = jwt_token[:10] if len(jwt_token) > 10 else jwt_token
    Logger.info(f"Attempting to update user profile with token prefix: {token_prefix}...")
    
    # Log update fields
    update_fields = {}
    if name is not None:
        update_fields["name"] = name
    if phone is not None:
        update_fields["phone"] = phone
    Logger.info("Update fields", {"fields": update_fields})
    
    # Call the update_user function
    result = update_user(jwt_token, name, phone)
    
    if "error" not in result:
        Logger.info("Successfully updated user profile", {"user_id": result.get("user_id", "unknown")})
    else:
        Logger.error("Failed to update user profile", {"error": result.get("error", "Unknown error")})
    
    return result

@mcp.tool("Terminate 2FA")
def terminate_2fa_tool(totp_token: str, skip_two_factor_auth: str = "True") -> Dict[str, Any]:
    """
    Terminates the 2FA check using the token from signin and retrieves the user info JWT.

    Args:
        totp_token: The authentication token obtained from the signin_tool.
        skip_two_factor_auth: Whether to skip the actual 2FA check (defaults to True).

    Returns:
        A dictionary containing the user info JWT or an error.
        Example success: {'user_info_token': 'eyJh...', 'user_id': '...', ...}
        Example error: {'error': 'API call failed...'}
    """
    try:
        # Convert the string input to boolean
        skip_bool = skip_two_factor_auth.lower() == 'true'
        Logger.info(f"Terminating 2FA check with skip_two_factor_auth={skip_bool}")

        # Configure API client with Bearer token authentication
        config = Configuration(
            host="http://localhost:8080", # Using localhost:8080 as required
            api_key={'Authorization': f'Bearer {totp_token}'}
        )
        api_client = ApiClient(configuration=config)

        # Prepare parameters for the low-level API call
        resource_path = '/user/2fa/terminate'
        method = 'GET'
        # Construct query parameters string
        query_string = f"skip_two_factor_auth={str(skip_bool).lower()}"
        # Construct the full URL
        request_url = config.host + resource_path + "?" + query_string

        # Prepare headers explicitly if needed (ApiClient might do this automatically via config.api_key)
        header_params = {
            'Authorization': config.api_key.get('Authorization')
        }

        # Log the API request
        ApiDebugger.log_request(method, request_url, header_params)

        # Use call_api with the corrected 'url' parameter
        Logger.debug(f"Making API call to {request_url}")
        start_time = time.time()  # For measuring API call duration
        response_data = api_client.call_api(
            method=method,
            url=request_url, # Pass the constructed URL
            header_params=header_params # Pass headers explicitly
        )
        elapsed_time = time.time() - start_time

        # Get status code
        status_code = getattr(response_data, 'status', 'N/A')
        Logger.debug(f"Received response with status code: {status_code}")
        
        # Explicitly read the response body because preload_content=False is used by rest.py
        raw_body_bytes = None
        try:
            if hasattr(response_data, 'read') and callable(response_data.read):
                raw_body_bytes = response_data.read()
                Logger.debug(f"Read {len(raw_body_bytes) if raw_body_bytes else 0} bytes from response")
            else:
                Logger.warning("Response data has no read() method")
        except Exception as read_exc:
             Logger.exception("Error reading response body", read_exc)
             return {"error": f"Failed to read response body: {read_exc}", "status_code": status_code}
        
        # Get headers if available
        headers = getattr(response_data, 'getheaders', lambda: {})()
        
        # Log the API response
        ApiDebugger.log_response(status_code, headers, raw_body_bytes, elapsed_time)

        # Check status code first and if raw_body_bytes exists
        if status_code == 200 and raw_body_bytes:
             try:
                 # Decode and parse JSON data
                 data_str = raw_body_bytes.decode('utf-8')
                 parsed_data = json.loads(data_str)
                 
                 if isinstance(parsed_data, dict) and 'token' in parsed_data:
                      # Rename token for clarity in the sequence
                      parsed_data["user_info_token"] = parsed_data.pop("token")
                      Logger.info("Successfully obtained user info token")
                      return parsed_data
                 else:
                      # If parsed_data is not a dict or doesn't have 'token', return an error
                      error_msg = "Parsed response body lacks expected 'token' field"
                      Logger.error(error_msg, {
                          "response_type": type(parsed_data).__name__, 
                          "response_data": str(parsed_data)
                      })
                      return {"error": error_msg, "response_type": type(parsed_data).__name__, "response_data": str(parsed_data)}

             except (json.JSONDecodeError, UnicodeDecodeError) as decode_error:
                  Logger.exception("Failed to decode/parse response data", decode_error)
                  return {"error": f"Failed to decode/parse response data: {decode_error}", "raw_response": repr(raw_body_bytes)}
        elif status_code == 200: # Status is 200 but raw_body_bytes is empty/None
            error_msg = "API returned 200 OK but response body was empty after read()."
            Logger.error(error_msg)
            return {"error": error_msg, "status_code": 200}
        else: # Status is not 200
             # Handle non-200 status, maybe try parsing error from body if available
             error_details = {}
             if raw_body_bytes:
                 try:
                     error_details = json.loads(raw_body_bytes.decode('utf-8'))
                 except Exception as e:
                     Logger.exception("Error parsing error response body", e)
                     error_details = {"raw_error": raw_body_bytes.decode('utf-8', errors='ignore')}
             error_msg = f"API call failed with status {status_code}"
             Logger.error(error_msg, {"details": error_details})
             return {"error": error_msg, "details": error_details}

    except ApiException as e:
        error_body = e.body
        try:
            # Try to parse the error body as JSON
            error_details = json.loads(error_body) if error_body else {}
        except json.JSONDecodeError:
            error_details = {"raw_error": str(error_body)} # Ensure body is string
        
        Logger.exception(f"API Exception with status {e.status}", e, {"details": error_details})
        return {"error": f"API call failed with status {e.status}", "details": error_details}
    except Exception as e:
        # Catch any other unexpected errors
        Logger.exception("Unexpected error in terminate_2fa_tool", e)
        return {"error": f"An unexpected error occurred: {str(e)}"}

@mcp.tool("List Business Profiles (v1)")
def list_business_profiles_tool(user_info_token: str, account_id: str, standard_api_key: str) -> Dict[str, Any]:
    """
    Lists all business profiles (v1 endpoint) associated with a merchant account using a user info JWT and API key.
    Uses localhost:8080 as the base URL.

    Args:
        user_info_token: The user info JWT obtained from the terminate_2fa_tool.
        account_id: The ID of the merchant account (e.g., merchant_id).
        standard_api_key: The standard merchant API key required for this endpoint.

    Returns:
        A dictionary containing a list of profiles or an error.
        Example success: {'profiles': [{'profile_id': '...', 'profile_name': '...'}, ...]}
        Example error: {'error': 'API call failed...'}
    """
    # Call the function from the business_profiles module
    result = list_business_profiles(user_info_token, standard_api_key, account_id)
    return result

@mcp.tool("Get Business Profile")
def get_business_profile_tool(user_info_token: str, account_id: str, profile_id: str, standard_api_key: str) -> Dict[str, Any]:
    """
    Gets details of a specific business profile using localhost:8080 as the base URL.

    Args:
        user_info_token: The user info JWT obtained from the terminate_2fa_tool.
        account_id: The ID of the merchant account (e.g., merchant_id).
        profile_id: The ID of the business profile to retrieve.
        standard_api_key: The standard merchant API key required for this endpoint.

    Returns:
        A dictionary containing the profile details or an error.
        Example success: {'profile': {'profile_id': '...', 'profile_name': '...'}}
        Example error: {'error': 'API call failed...'}
    """
    # Call the function from the business_profiles module
    result = get_business_profile(user_info_token, standard_api_key, account_id, profile_id)
    return result

@mcp.tool("Create Business Profile")
def create_business_profile_tool(
    user_info_token: str, 
    account_id: str, 
    standard_api_key: str,
    profile_name: str,
    description: str = None,
    return_url: str = None,
    webhook_url: str = None,
    webhook_version: str = None,
    metadata: dict = None
) -> Dict[str, Any]:
    """
    Creates a new business profile using localhost:8080 as the base URL.

    Args:
        user_info_token: The user info JWT obtained from the terminate_2fa_tool.
        account_id: The ID of the merchant account (e.g., merchant_id).
        standard_api_key: The standard merchant API key required for this endpoint.
        profile_name: Name for the new business profile.
        description: Optional description for the business profile.
        return_url: Optional return URL for the business profile.
        webhook_url: Optional webhook URL for the business profile.
        webhook_version: Optional webhook version for the business profile.
        metadata: Optional metadata for the business profile.

    Returns:
        A dictionary containing the created profile details or an error.
        Example success: {'profile': {'profile_id': '...', 'profile_name': '...'}}
        Example error: {'error': 'API call failed...'}
    """
    # Call the function from the business_profiles module
    result = create_business_profile(
        user_info_token, 
        standard_api_key, 
        account_id, 
        profile_name,
        description,
        return_url,
        webhook_url,
        webhook_version,
        metadata
    )
    return result

@mcp.tool("Update Business Profile")
def update_business_profile_tool(
    user_info_token: str, 
    account_id: str, 
    profile_id: str,
    standard_api_key: str, # TODO: Review if 'hyperswitch' key is always needed/intended here
    profile_name: str = None,
    # description: str = None, # Description likely handled via metadata
    return_url: str = None,
    webhook_url: str = None,
    webhook_version: str = None,
    metadata: dict = None,
    # Pass defaults for potentially required fields based on successful create tests
    enable_payment_response_hash: bool = True,
    redirect_to_merchant_with_http_post: bool = False,
    use_billing_as_payment_method_billing: bool = True,
    session_expiry: int = 900 
) -> Dict[str, Any]:
    """
    Updates an existing business profile using localhost:8080 as the base URL.
    Calls the underlying business_profiles.update_business_profile function.

    Args:
        user_info_token: The user info JWT obtained from the terminate_2fa_tool.
        account_id: The ID of the merchant account (e.g., merchant_id).
        profile_id: The ID of the business profile to update.
        standard_api_key: The standard merchant API key required for this endpoint. 
                          (Note: 'hyperswitch' key worked in testing).
        profile_name: Optional new name for the business profile.
        # description: Optional (Likely ignored - use metadata?).
        return_url: Optional new return URL for the business profile.
        webhook_url: Optional new webhook URL.
        webhook_version: Optional new webhook version.
        metadata: Optional new metadata (replaces existing).
        enable_payment_response_hash: Optional.
        redirect_to_merchant_with_http_post: Optional.
        use_billing_as_payment_method_billing: Optional.
        session_expiry: Optional.

    Returns:
        A dictionary containing the updated profile details or an error.
    """
    # Simply call the refactored function from the business_profiles module
    return update_business_profile(
        jwt_token=user_info_token,
        api_key=standard_api_key,
        account_id=account_id,
        profile_id=profile_id,
        profile_name=profile_name,
        # description=description, # Excluded for now
        return_url=return_url,
        webhook_url=webhook_url,
        webhook_version=webhook_version,
        metadata=metadata,
        enable_payment_response_hash=enable_payment_response_hash,
        redirect_to_merchant_with_http_post=redirect_to_merchant_with_http_post,
        use_billing_as_payment_method_billing=use_billing_as_payment_method_billing,
        session_expiry=session_expiry
    )

@mcp.tool("Delete Business Profile")    
def delete_business_profile_tool(
    user_info_token: str, 
    account_id: str, 
    profile_id: str,
    standard_api_key: str
) -> Dict[str, Any]:
    """
    Deletes a business profile using localhost:8080 as the base URL.

    Args:
        user_info_token: The user info JWT obtained from the terminate_2fa_tool.
        account_id: The ID of the merchant account (e.g., merchant_id).
        profile_id: The ID of the business profile to delete.
        standard_api_key: The standard merchant API key required for this endpoint.

    Returns:
        A dictionary containing the deletion status or an error.
        Example success: {'success': true, 'message': 'Profile deleted successfully'}
        Example error: {'error': 'API call failed...'}
    """
    # Call the function from the business_profiles module
    result = delete_business_profile(user_info_token, standard_api_key, account_id, profile_id)
    return result

# --- New User Management Tools Start --- 

@mcp.tool("Change User Password")
def change_password_tool(jwt_token: str, current_password: str, new_password: str) -> Dict[str, Any]:
    """
    Changes the currently authenticated user's password.
    
    Args:
        jwt_token: The user's current authentication JWT.
        current_password: The user's existing password.
        new_password: The desired new password.
        
    Returns:
        A dictionary indicating success or failure.
    """
    Logger.info("Executing change_password_tool")
    # Call the function from the user module
    return change_password(jwt_token, current_password, new_password)

@mcp.tool("Initiate Password Reset")
def initiate_password_reset_tool(email: str) -> Dict[str, Any]:
    """
    Starts the password reset process for a user by sending them a reset token.
    
    Args:
        email: The email address of the user requesting the reset.
        
    Returns:
        A dictionary indicating success or failure of the initiation process.
    """
    Logger.info(f"Executing initiate_password_reset_tool for email: {email}")
    # Call the function from the user module
    return initiate_password_reset(email)

@mcp.tool("Confirm Password Reset")
def reset_password_confirm_tool(reset_token: str, new_password: str) -> Dict[str, Any]:
    """
    Completes the password reset process using the token and sets the new password.
    
    Args:
        reset_token: The password reset token sent to the user's email.
        new_password: The new password to set for the user.
        
    Returns:
        A dictionary indicating success or failure of the password reset confirmation.
    """
    Logger.info("Executing reset_password_confirm_tool")
    # Call the function from the user module
    return reset_password_confirm(reset_token, new_password)

@mcp.tool("Verify User Email")
def verify_email_tool(jwt_token: str, verification_token: str) -> Dict[str, Any]:
    """
    Verifies a user's email address using a verification token.
    
    Args:
        jwt_token: The user's JWT token for authorization.
        verification_token: The token sent to the user's email for verification.
        
    Returns:
        A dictionary indicating success or failure of the email verification.
    """
    Logger.info("Executing verify_email_tool")
    # Call the function from the user module, passing jwt_token
    return verify_email(jwt_token, verification_token)

# --- New User Management Tools End --- 

if __name__ == "__main__":
    # Initialize logging
    initialize_logging(LogLevel.DEBUG)
    Logger.info("Starting hyperswitch-mcp server")
    
    try:
        Logger.info("Running MCP server with stdio transport")
        mcp.run(transport="stdio")
    except Exception as e:
        # Print the standard JSON error for the MCP client
        print(json.dumps({"error": str(e)}))
        # Log the exception
        Logger.exception("Fatal error in MCP server", e)