"""
Business Profiles Module for Hyperswitch MCP

This module provides functions for managing business profiles in Hyperswitch.
All API calls use localhost:8080 as the base URL.
"""

import json
import logging
import time
from typing import Dict, Any, List, Optional, Union

import requests
from requests.exceptions import RequestException, Timeout

# Set up logging
logger = logging.getLogger(__name__)

# Fixed base URL as per requirements
BASE_URL = "http://localhost:8080"

class BusinessProfile:
    """Represents a business profile in the Hyperswitch platform."""
    
    def __init__(
        self,
        profile_id: str = None,
        profile_name: str = None,
        description: str = None,
        created_at: str = None,
        modified_at: str = None,
        return_url: str = None,
        payment_response_hash_key: str = None,
        webhook_url: str = None,
        webhook_version: str = None,
        webhook_username: str = None,
        webhook_password: str = None,
        webhook_api_key: str = None,
        metadata: dict = None,
        payment_methods_enabled: List[str] = None,
        **kwargs
    ):
        self.profile_id = profile_id
        self.profile_name = profile_name
        self.description = description
        self.created_at = created_at
        self.modified_at = modified_at
        self.return_url = return_url
        self.payment_response_hash_key = payment_response_hash_key
        self.webhook_url = webhook_url
        self.webhook_version = webhook_version
        self.webhook_username = webhook_username
        self.webhook_password = webhook_password
        self.webhook_api_key = webhook_api_key
        self.metadata = metadata or {}
        self.payment_methods_enabled = payment_methods_enabled or []
        self.__dict__.update(kwargs)
    
    def to_dict(self) -> dict:
        """Convert the business profile to a dictionary."""
        return {k: v for k, v in self.__dict__.items() if v is not None}
    
    @classmethod
    def from_dict(cls, data: dict) -> 'BusinessProfile':
        """Create a BusinessProfile instance from a dictionary."""
        return cls(**data)


def list_business_profiles(
    jwt_token: str, 
    api_key: str, 
    account_id: str
) -> Dict[str, Any]:
    """
    List all business profiles for an account.
    
    Args:
        jwt_token: JWT authentication token
        api_key: API key for the account
        account_id: Account ID (merchant ID)
        
    Returns:
        A dictionary containing the list of profiles or an error
    """
    url = f"{BASE_URL}/account/{account_id}/business_profile"
    
    # Set up headers with both JWT token and API key
    headers = {
        "Authorization": f"Bearer {jwt_token}",
        "api-key": api_key,
        "Accept": "application/json"
    }
    
    logger.info(f"Listing business profiles for account: {account_id}")
    start_time = time.time()
    
    try:
        response = requests.get(url, headers=headers, timeout=10)
        elapsed_time = time.time() - start_time
        
        logger.debug(f"Response received in {elapsed_time:.2f}s with status code: {response.status_code}")
        
        # Parse response based on status code
        if response.status_code == 200:
            profiles_data = response.json()
            
            # Convert to BusinessProfile objects if it's a list
            if isinstance(profiles_data, list):
                profiles = [BusinessProfile.from_dict(profile) for profile in profiles_data]
                return {
                    "profiles": [profile.to_dict() for profile in profiles],
                    "count": len(profiles),
                    "success": True
                }
            else:
                # If it's not a list, return the data as is
                return {
                    "data": profiles_data,
                    "success": True
                }
        else:
            # Try to get error details from response
            try:
                error_data = response.json()
                error_message = error_data.get("message", "Unknown error")
                error_code = error_data.get("code", "unknown")
            except ValueError:
                error_message = response.text or "Unknown error"
                error_code = "unknown"
            
            logger.error(f"Failed to list business profiles: {error_message} (code: {error_code})")
            return {
                "error": error_message,
                "error_code": error_code,
                "status_code": response.status_code,
                "success": False
            }
            
    except Timeout:
        logger.error("Request timed out when listing business profiles")
        return {
            "error": "Request timed out",
            "success": False
        }
    except RequestException as e:
        logger.error(f"Request exception when listing business profiles: {str(e)}")
        return {
            "error": str(e),
            "success": False
        }
    except Exception as e:
        logger.exception(f"Unexpected error when listing business profiles: {str(e)}")
        return {
            "error": f"Unexpected error: {str(e)}",
            "success": False
        }


def get_business_profile(
    jwt_token: str, 
    api_key: str, 
    account_id: str, 
    profile_id: str
) -> Dict[str, Any]:
    """
    Get details of a specific business profile.
    
    Args:
        jwt_token: JWT authentication token
        api_key: API key for the account
        account_id: Account ID (merchant ID)
        profile_id: ID of the profile to retrieve
        
    Returns:
        A dictionary containing the profile details or an error
    """
    url = f"{BASE_URL}/account/{account_id}/business_profile/{profile_id}"
    
    headers = {
        "Authorization": f"Bearer {jwt_token}",
        "api-key": api_key,
        "Accept": "application/json"
    }
    
    logger.info(f"Getting business profile {profile_id} for account: {account_id}")
    
    try:
        response = requests.get(url, headers=headers, timeout=10)
        
        if response.status_code == 200:
            profile_data = response.json()
            profile = BusinessProfile.from_dict(profile_data)
            return {
                "profile": profile.to_dict(),
                "success": True
            }
        else:
            # Try to get error details from response
            try:
                error_data = response.json()
                error_message = error_data.get("message", "Unknown error")
                error_code = error_data.get("code", "unknown")
            except ValueError:
                error_message = response.text or "Unknown error"
                error_code = "unknown"
            
            logger.error(f"Failed to get business profile: {error_message} (code: {error_code})")
            return {
                "error": error_message,
                "error_code": error_code,
                "status_code": response.status_code,
                "success": False
            }
            
    except RequestException as e:
        logger.error(f"Request exception when getting business profile: {str(e)}")
        return {
            "error": str(e),
            "success": False
        }
    except Exception as e:
        logger.exception(f"Unexpected error when getting business profile: {str(e)}")
        return {
            "error": f"Unexpected error: {str(e)}",
            "success": False
        }


def create_business_profile(
    jwt_token: str, 
    api_key: str, 
    account_id: str, 
    profile_name: str,
    description: str = None,
    return_url: str = None,
    webhook_url: str = None,
    webhook_version: str = None,
    metadata: dict = None,
    # Add missing fields based on test script
    enable_payment_response_hash: bool = True,
    redirect_to_merchant_with_http_post: bool = False,
    use_billing_as_payment_method_billing: bool = True,
    session_expiry: int = 900 
) -> Dict[str, Any]:
    """
    Create a new business profile.
    
    Args:
        jwt_token: JWT authentication token
        api_key: API key for the account
        account_id: Account ID (merchant ID)
        profile_name: Name for the new profile
        description: Optional description
        return_url: Optional return URL
        webhook_url: Optional webhook URL
        webhook_version: Optional webhook version
        metadata: Optional metadata dictionary
        enable_payment_response_hash: Optional (defaults to True)
        redirect_to_merchant_with_http_post: Optional (defaults to False)
        use_billing_as_payment_method_billing: Optional (defaults to True)
        session_expiry: Optional session expiry time in seconds (defaults to 900)
        
    Returns:
        A dictionary containing the created profile details or an error
    """
    url = f"{BASE_URL}/account/{account_id}/business_profile"
    
    headers = {
        "Authorization": f"Bearer {jwt_token}",
        "api-key": api_key,
        "Content-Type": "application/json",
        "Accept": "application/json"
    }
    
    # Prepare request data including new fields
    profile_data = {
        "profile_name": profile_name,
        "enable_payment_response_hash": enable_payment_response_hash,
        "redirect_to_merchant_with_http_post": redirect_to_merchant_with_http_post,
        "use_billing_as_payment_method_billing": use_billing_as_payment_method_billing,
        "session_expiry": session_expiry
    }
    
    # Add optional fields if provided
    if return_url:
        profile_data["return_url"] = return_url
        
    # Structure webhook details if provided
    webhook_details = {}
    if webhook_url:
        webhook_details["webhook_url"] = webhook_url
    if webhook_version:
        webhook_details["webhook_version"] = webhook_version
        
    if webhook_details: # Only add the object if it has content
        profile_data["webhook_details"] = webhook_details
        
    logger.info(f"Creating business profile '{profile_name}' for account: {account_id}")
    
    try:
        response = requests.post(url, headers=headers, json=profile_data, timeout=10)
        
        if response.status_code in (200, 201):
            profile_data = response.json()
            profile = BusinessProfile.from_dict(profile_data)
            return {
                "profile": profile.to_dict(),
                "success": True
            }
        else:
            # Try to get error details from response
            try:
                error_data = response.json()
                error_message = error_data.get("message", "Unknown error")
                error_code = error_data.get("code", "unknown")
            except ValueError:
                error_message = response.text or "Unknown error"
                error_code = "unknown"
            
            logger.error(f"Failed to create business profile: {error_message} (code: {error_code})")
            return {
                "error": error_message,
                "error_code": error_code,
                "status_code": response.status_code,
                "success": False
            }
            
    except RequestException as e:
        logger.error(f"Request exception when creating business profile: {str(e)}")
        return {
            "error": str(e),
            "success": False
        }
    except Exception as e:
        logger.exception(f"Unexpected error when creating business profile: {str(e)}")
        return {
            "error": f"Unexpected error: {str(e)}",
            "success": False
        }


def update_business_profile_tool(
    user_info_token: str, 
    account_id: str, 
    profile_id: str,
    standard_api_key: str,
    profile_name: str = None,
    description: str = None,
    return_url: str = None,
    webhook_url: str = None,
    webhook_version: str = None,
    metadata: dict = None
) -> Dict[str, Any]:
    url = f"{BASE_URL}/account/{account_id}/business_profile/{profile_id}"
    
    headers = {
        "Authorization": f"Bearer {user_info_token}",
        "api-key": standard_api_key,
        "Content-Type": "application/json",
        "Accept": "application/json"
    }
    
    # Prepare request data
    profile_data = {}
    if profile_name: profile_data["profile_name"] = profile_name
    if description is not None: profile_data["description"] = description
    if return_url is not None: profile_data["return_url"] = return_url
    if webhook_url is not None: profile_data["webhook_url"] = webhook_url
    if webhook_version is not None: profile_data["webhook_version"] = webhook_version
    if metadata is not None: profile_data["metadata"] = metadata
    
    # Try POST instead of PUT
    response = requests.post(url, headers=headers, json=profile_data, timeout=10)
    
    if response.status_code == 200:
        profile_data = response.json()
        profile = BusinessProfile.from_dict(profile_data)
        return {
            "profile": profile.to_dict(),
            "success": True
        }
    else:
        # Try to get error details from response
        try:
            error_data = response.json()
            error_message = error_data.get("message", "Unknown error")
            error_code = error_data.get("code", "unknown")
        except ValueError:
            error_message = response.text or "Unknown error"
            error_code = "unknown"
        
        logger.error(f"Failed to update business profile: {error_message} (code: {error_code})")
        return {
            "error": error_message,
            "error_code": error_code,
            "status_code": response.status_code,
            "success": False
        }


def update_business_profile(
    jwt_token: str,
    api_key: str,
    account_id: str, 
    profile_id: str,
    profile_name: str = None,
    # description: str = None, # Description likely handled via metadata
    return_url: str = None,
    webhook_url: str = None,
    webhook_version: str = None,
    metadata: dict = None,
    # Add fields that might be required based on create/test samples
    enable_payment_response_hash: bool = True,
    redirect_to_merchant_with_http_post: bool = False,
    use_billing_as_payment_method_billing: bool = True,
    session_expiry: int = 900 
) -> Dict[str, Any]:
    """
    Updates an existing business profile using the backend API.
    Constructs payload only from provided fields + defaults and uses POST.
    
    Args:
        jwt_token: JWT authentication token
        api_key: API key for the account
        account_id: Account ID (merchant ID)
        profile_id: ID of the profile to update
        profile_name: Optional new name
        return_url: Optional new return URL
        webhook_url: Optional new webhook URL
        webhook_version: Optional new webhook version
        metadata: Optional new metadata (replaces existing)
        enable_payment_response_hash: Optional
        redirect_to_merchant_with_http_post: Optional
        use_billing_as_payment_method_billing: Optional
        session_expiry: Optional
        
    Returns:
        A dictionary containing the updated profile details or an error.
    """
    url = f"{BASE_URL}/account/{account_id}/business_profile/{profile_id}"
    
    headers = {
        "Authorization": f"Bearer {jwt_token}",
        "api-key": api_key,
        "Content-Type": "application/json",
        "Accept": "application/json"
    }

    # Build payload ONLY from provided arguments + defaults from create sample
    update_payload = {
        "enable_payment_response_hash": enable_payment_response_hash,
        "redirect_to_merchant_with_http_post": redirect_to_merchant_with_http_post,
        "use_billing_as_payment_method_billing": use_billing_as_payment_method_billing,
        "session_expiry": session_expiry
    }
    updated_fields_count = 0
    
    if profile_name is not None:
        update_payload["profile_name"] = profile_name
        updated_fields_count += 1
    if return_url is not None:
        update_payload["return_url"] = return_url
        updated_fields_count += 1

    webhook_details = {}
    if webhook_url is not None:
        webhook_details["webhook_url"] = webhook_url
        updated_fields_count += 1
    if webhook_version is not None:
        webhook_details["webhook_version"] = webhook_version
        updated_fields_count += 1
    if webhook_details:
         update_payload["webhook_details"] = webhook_details

    if metadata is not None:
        update_payload["metadata"] = metadata
        updated_fields_count += 1

    # An update should modify at least one optional field
    if updated_fields_count == 0:
         return {"error": "No actual update fields provided to modify"}

    logger.info(f"Updating business profile {profile_id} for account: {account_id} with specific fields.")
    logger.debug(f"Update payload: {update_payload}")

    try:
        response = requests.post(url, headers=headers, json=update_payload, timeout=15)
        
        if response.status_code == 200:
            try:
                profile_data = response.json()
                # Parse response back through the class for consistency
                profile = BusinessProfile.from_dict(profile_data)
                logger.info("Successfully updated business profile")
                return {
                    "profile": profile.to_dict(),
                    "success": True
                }
            except (ValueError, TypeError) as json_err:
                logger.error(f"Update successful (200 OK) but failed to parse response JSON: {json_err}", {"response_text": response.text[:200]})
                return {"error": "Update succeeded but failed to parse response", "status_code": 200, "success": False}
        else:
            try:
                error_data = response.json()
                error_message = error_data.get("message", "Unknown error")
                error_code = error_data.get("code", "unknown")
            except ValueError:
                error_message = response.text or "Unknown error"
                error_code = "unknown"
            
            logger.error(f"Failed to update business profile: {error_message} (code: {error_code}, status: {response.status_code})")
            return {
                "error": error_message,
                "error_code": error_code,
                "status_code": response.status_code,
                "success": False
            }
            
    except RequestException as e:
        logger.error(f"Request exception when updating business profile: {str(e)}")
        return {"error": str(e), "success": False}
    except Exception as e:
        logger.exception(f"Unexpected error when updating business profile: {str(e)}")
        return {"error": f"Unexpected error: {str(e)}", "success": False}


def delete_business_profile(
    jwt_token: str, 
    api_key: str, 
    account_id: str,
    profile_id: str
) -> Dict[str, Any]:
    """
    Delete a business profile.
    
    Args:
        jwt_token: JWT authentication token
        api_key: API key for the account
        account_id: Account ID (merchant ID)
        profile_id: ID of the profile to delete
        
    Returns:
        A dictionary containing the deletion status or an error
    """
    url = f"{BASE_URL}/account/{account_id}/business_profile/{profile_id}"
    
    headers = {
        "Authorization": f"Bearer {jwt_token}",
        "api-key": api_key,
        "Accept": "application/json"
    }
    
    logger.info(f"Deleting business profile {profile_id} for account: {account_id}")
    
    try:
        response = requests.delete(url, headers=headers, timeout=10)
        
        if response.status_code in (200, 204):
            # Try to parse response as JSON if available
            try:
                result = response.json()
            except ValueError:
                # If no JSON response, create a simple success result
                result = {"message": "Profile deleted successfully"}
            
            return {
                "result": result,
                "success": True
            }
        else:
            # Try to get error details from response
            try:
                error_data = response.json()
                error_message = error_data.get("message", "Unknown error")
                error_code = error_data.get("code", "unknown")
            except ValueError:
                error_message = response.text or "Unknown error"
                error_code = "unknown"
            
            logger.error(f"Failed to delete business profile: {error_message} (code: {error_code})")
            return {
                "error": error_message,
                "error_code": error_code,
                "status_code": response.status_code,
                "success": False
            }
            
    except RequestException as e:
        logger.error(f"Request exception when deleting business profile: {str(e)}")
        return {
            "error": str(e),
            "success": False
        }
    except Exception as e:
        logger.exception(f"Unexpected error when deleting business profile: {str(e)}")
        return {
            "error": f"Unexpected error: {str(e)}",
            "success": False
        }


def update_user(jwt_token: str, name: str = None, phone: str = None) -> Dict[str, Any]:
    url = "http://localhost:8080/user/update"
    headers = {
        "Authorization": f"Bearer {jwt_token}",
        "Content-Type": "application/json",
        "Accept": "application/json"
    }
    
    payload = {}
    if name is not None: payload["name"] = name
    if phone is not None: payload["phone"] = phone
    
    # Try POST instead of PUT
    response = requests.post(url, headers=headers, json=payload)
    
    if response.status_code == 200:
        return {"success": True}
    else:
        # Try to get error details from response
        try:
            error_data = response.json()
            error_message = error_data.get("message", "Unknown error")
            error_code = error_data.get("code", "unknown")
        except ValueError:
            error_message = response.text or "Unknown error"
            error_code = "unknown"
        
        logger.error(f"Failed to update user: {error_message} (code: {error_code})")
        return {
            "error": error_message,
            "error_code": error_code,
            "status_code": response.status_code,
            "success": False
        } 