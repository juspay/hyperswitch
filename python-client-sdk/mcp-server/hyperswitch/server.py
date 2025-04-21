from typing import Dict, Any, List, Optional
from mcp.server.fastmcp import FastMCP
from hyperswitch import ApiClient, Configuration
from hyperswitch.api.api_key_api import APIKeyApi
from hyperswitch.api.merchant_account_api import MerchantAccountApi
from hyperswitch.api.profile_api import ProfileApi
from hyperswitch.api.customers_api import CustomersApi
from hyperswitch.models.create_api_key_request import CreateApiKeyRequest
from hyperswitch.models.update_api_key_request import UpdateApiKeyRequest
from hyperswitch.models.api_key_expiration import ApiKeyExpiration
from hyperswitch.models.merchant_account_create import MerchantAccountCreate
from hyperswitch.models.merchant_account_update import MerchantAccountUpdate
from hyperswitch.models.merchant_details import MerchantDetails
from hyperswitch.models.address_details import AddressDetails
from hyperswitch.models.country_alpha2 import CountryAlpha2
from hyperswitch.models.profile_create import ProfileCreate
from hyperswitch.models.webhook_details import WebhookDetails
from hyperswitch.models.customer_request import CustomerRequest
from hyperswitch.models.customer_update_request import CustomerUpdateRequest
from hyperswitch.api.merchant_connector_account_api import MerchantConnectorAccountApi
from hyperswitch.models.merchant_connector_details import MerchantConnectorDetails
from hyperswitch.models.merchant_connector_create import MerchantConnectorCreate
from hyperswitch.models.merchant_connector_update import MerchantConnectorUpdate
from hyperswitch.models.primary_business_details import PrimaryBusinessDetails
from hyperswitch.models.routing_algorithm import RoutingAlgorithm
from hyperswitch.models.routing_algorithm_kind import RoutingAlgorithmKind
from hyperswitch.models.routable_connector_choice import RoutableConnectorChoice
from hyperswitch.models.connector_volume_split import ConnectorVolumeSplit
from hyperswitch.api import RoutingApi
from hyperswitch.models import (
    RoutingConfigRequest, 
    RoutingAlgorithm,
    RoutableConnectorChoice,
    ConnectorVolumeSplit,
    MerchantRoutingAlgorithm,
    RoutingDictionaryRecord,
    LinkedRoutingConfigRetrieveResponse,
    ProfileDefaultRoutingConfig
)
import os

# Initialize the MCP server
mcp = FastMCP('hyperswitch-mcp')

def get_admin_auth() -> ApiClient:
    """
    Get API client with admin authentication.
    
    Returns:
        ApiClient configured with admin authentication
    """
    config = Configuration(
        host="http://localhost:8080",
        api_key={"admin_api_key": os.getenv("ADMIN_API_KEY")}
    )
    return ApiClient(configuration=config)

def get_api_key_auth() -> ApiClient:
    """
    Get API client with hardcoded API key authentication.
    
    Returns:
        ApiClient configured with API key authentication
    """
    config = Configuration(
        host="http://localhost:8080",
        api_key={"api_key": "test_admin"}
    )
    return ApiClient(configuration=config)

def get_merchant_api_key_auth(merchant_id: str) -> ApiClient:
    """
    Get API client with merchant's API key authentication.
    
    Args:
        merchant_id: The ID of the merchant to get API key for
        
    Returns:
        ApiClient configured with merchant's API key authentication
    """
    # Use the existing API key we created earlier
    config = Configuration(
        host="http://localhost:8080",
        api_key={"api_key": "snd_D3e6iF5eO6ptrNGZx07s5bCI2WUDZCvkKutNk39CKsRnXjVASqkCer2ejOjT0rcW"}
    )
    return ApiClient(configuration=config)

@mcp.tool("Say hello to someone")
def say_hello(name: str) -> Dict[str, Any]:
    """
    A simple hello world tool that greets the given name.
    
    Args:
        name: The name to greet
        
    Returns:
        A dictionary containing the greeting message
    """
    return {"message": f"Hello, {name}!"}

@mcp.tool("Create an API Key")
def create_api_key(merchant_id: str, name: str, description: str, expiration: str = "never") -> Dict[str, Any]:
    """
    Create a new API key for a merchant.
    
    Args:
        merchant_id: The ID of the merchant
        name: Name for the API key
        description: Description of the API key
        expiration: Expiration setting ("never" or a specific date)
        
    Returns:
        A dictionary containing the API key details
    """
    client = get_admin_auth()
    api_key_api = APIKeyApi(client)
    
    api_key_request = CreateApiKeyRequest(
        merchant_id=merchant_id,
        name=name,
        description=description,
        expiration=ApiKeyExpiration(expiration)
    )
    
    response = api_key_api.create_an_api_key(merchant_id, api_key_request)
    return {
        "key_id": response.key_id,
        "api_key": response.api_key,
        "merchant_id": response.merchant_id,
        "name": response.name,
        "description": response.description,
        "expiration": response.expiration.actual_instance
    }

@mcp.tool("Retrieve an API Key")
def retrieve_api_key(merchant_id: str, key_id: str) -> Dict[str, Any]:
    """
    Retrieve details of an existing API key.
    
    Args:
        merchant_id: The ID of the merchant
        key_id: The ID of the API key to retrieve
        
    Returns:
        A dictionary containing the API key details
    """
    client = get_admin_auth()
    api_key_api = APIKeyApi(client)
    
    response = api_key_api.retrieve_an_api_key(merchant_id, key_id)
    return {
        "key_id": response.key_id,
        "merchant_id": response.merchant_id,
        "name": response.name,
        "description": response.description,
        "expiration": response.expiration.actual_instance
    }

@mcp.tool("Update an API Key")
def update_api_key(merchant_id: str, key_id: str, name: str, description: str, expiration: str = "never") -> Dict[str, Any]:
    """
    Update an existing API key.
    
    Args:
        merchant_id: The ID of the merchant
        key_id: The ID of the API key to update
        name: New name for the API key
        description: New description for the API key
        expiration: New expiration setting ("never" or a specific date)
        
    Returns:
        A dictionary containing the updated API key details
    """
    client = get_admin_auth()
    api_key_api = APIKeyApi(client)
    
    update_request = UpdateApiKeyRequest(
        merchant_id=merchant_id,
        name=name,
        description=description,
        expiration=ApiKeyExpiration(expiration)
    )
    
    response = api_key_api.update_an_api_key(merchant_id, key_id, update_request)
    return {
        "key_id": response.key_id,
        "merchant_id": response.merchant_id,
        "name": response.name,
        "description": response.description,
        "expiration": response.expiration.actual_instance
    }

@mcp.tool("Create a Merchant Account")
def create_merchant_account(
    merchant_id: str,
    merchant_name: str,
    primary_email: str,
    primary_phone: str,
    primary_contact: str,
    line1: str,
    city: str,
    state: str,
    zip_code: str,
    country: str,
    business: str = "default",
    return_url: str = "https://example.com/return"
) -> Dict[str, Any]:
    """
    Create a new merchant account.
    
    Args:
        merchant_id: Unique identifier for the merchant
        merchant_name: Name of the merchant
        primary_email: Primary email address
        primary_phone: Primary phone number
        primary_contact: Primary contact person
        line1: Address line 1
        city: City
        state: State
        zip_code: ZIP code
        country: Country code (e.g., "US")
        business: Business type/label for the merchant (default: "default")
        return_url: Return URL for the merchant
        
    Returns:
        A dictionary containing the merchant account details
    """
    client = get_admin_auth()
    merchant_api = MerchantAccountApi(client)
    
    address = AddressDetails(
        line1=line1,
        city=city,
        state=state,
        zip_=zip_code,
        country=CountryAlpha2(country)
    )
    
    merchant_details = MerchantDetails(
        primary_email=primary_email,
        primary_phone=primary_phone,
        primary_contact_person=primary_contact,
        address=address
    )
    
    # Create primary business details with provided business label
    primary_business_details = [
        PrimaryBusinessDetails(
            country=CountryAlpha2(country),
            business=business
        )
    ]
    
    merchant_request = MerchantAccountCreate(
        merchant_id=merchant_id,
        merchant_name=merchant_name,
        merchant_details=merchant_details,
        return_url=return_url,
        webhook_details={
            "webhook_version": "1.0.1",
            "payment_created_enabled": True,
            "payment_succeeded_enabled": True,
            "payment_failed_enabled": True
        },
        sub_merchants_enabled=False,
        primary_business_details=primary_business_details
    )
    
    merchant = merchant_api.create_a_merchant_account(merchant_account_create=merchant_request)
    return {
        "merchant_id": merchant.merchant_id,
        "merchant_name": merchant.merchant_name,
        "organization_id": merchant.organization_id,
        "publishable_key": merchant.publishable_key
    }

@mcp.tool("Retrieve a Merchant Account")
def retrieve_merchant_account(merchant_id: str) -> Dict[str, Any]:
    """
    Retrieve details of a merchant account.
    
    Args:
        merchant_id: The ID of the merchant account to retrieve
        
    Returns:
        A dictionary containing the merchant account details
    """
    client = get_admin_auth()
    merchant_api = MerchantAccountApi(client)
    
    merchant = merchant_api.retrieve_a_merchant_account(merchant_id)
    return {
        "merchant_id": merchant.merchant_id,
        "merchant_name": merchant.merchant_name,
        "organization_id": merchant.organization_id,
        "publishable_key": merchant.publishable_key
    }

@mcp.tool("Update a Merchant Account")
def update_merchant_account(
    merchant_id: str,
    merchant_name: str,
    primary_email: str,
    primary_phone: str,
    primary_contact: str
) -> Dict[str, Any]:
    """
    Update an existing merchant account.
    
    Args:
        merchant_id: The ID of the merchant account to update
        merchant_name: New name for the merchant
        primary_email: New primary email address
        primary_phone: New primary phone number
        primary_contact: New primary contact person
        
    Returns:
        A dictionary containing the updated merchant account details
    """
    client = get_admin_auth()
    merchant_api = MerchantAccountApi(client)
    
    update_request = MerchantAccountUpdate(
        merchant_id=merchant_id,
        merchant_name=merchant_name,
        merchant_details=MerchantDetails(
            primary_email=primary_email,
            primary_phone=primary_phone,
            primary_contact_person=primary_contact
        )
    )
    
    merchant = merchant_api.update_a_merchant_account(merchant_id, merchant_account_update=update_request)
    return {
        "merchant_id": merchant.merchant_id,
        "merchant_name": merchant.merchant_name,
        "organization_id": merchant.organization_id,
        "publishable_key": merchant.publishable_key
    }

@mcp.tool("Create a Profile")
def create_profile(
    merchant_id: str,
    profile_name: str,
    return_url: str = "https://example.com/return",
    webhook_url: str = "https://example.com/webhook",
    webhook_version: str = "1.0.1",
    enable_payment_response_hash: bool = True,
    redirect_to_merchant_with_http_post: bool = False
) -> Dict[str, Any]:
    """
    Create a new profile for a merchant.
    
    Args:
        merchant_id: The ID of the merchant
        profile_name: Name for the profile
        return_url: Return URL for the profile
        webhook_url: Webhook URL for notifications
        webhook_version: Version of the webhook
        enable_payment_response_hash: Whether to enable payment response hash
        redirect_to_merchant_with_http_post: Whether to redirect to merchant with HTTP POST
        
    Returns:
        A dictionary containing the profile details
    """
    client = get_api_key_auth()
    profile_api = ProfileApi(client)
    
    webhook_details = WebhookDetails(
        webhook_url=webhook_url,
        webhook_version=webhook_version,
        webhook_username="test_username",
        webhook_password="test_password",
        payment_created_enabled=True,
        payment_succeeded_enabled=True,
        payment_failed_enabled=True
    )
    
    profile_request = ProfileCreate(
        profile_name=profile_name,
        return_url=return_url,
        enable_payment_response_hash=enable_payment_response_hash,
        redirect_to_merchant_with_http_post=redirect_to_merchant_with_http_post,
        webhook_details=webhook_details
    )
    
    response = profile_api.create_a_profile(merchant_id, profile_request)
    return {
        "profile_id": response.profile_id,
        "profile_name": response.profile_name,
        "return_url": response.return_url,
        "webhook_details": {
            "webhook_url": response.webhook_details.webhook_url,
            "webhook_version": response.webhook_details.webhook_version
        },
        "enable_payment_response_hash": response.enable_payment_response_hash,
        "redirect_to_merchant_with_http_post": response.redirect_to_merchant_with_http_post
    }

@mcp.tool("Retrieve a Profile")
def retrieve_profile(merchant_id: str, profile_id: str) -> Dict[str, Any]:
    """
    Retrieve details of an existing profile.
    
    Args:
        merchant_id: The ID of the merchant
        profile_id: The ID of the profile to retrieve
        
    Returns:
        A dictionary containing the profile details
    """
    client = get_api_key_auth()
    profile_api = ProfileApi(client)
    
    response = profile_api.retrieve_a_profile(merchant_id, profile_id)
    return {
        "profile_id": response.profile_id,
        "profile_name": response.profile_name,
        "return_url": response.return_url,
        "webhook_details": {
            "webhook_url": response.webhook_details.webhook_url,
            "webhook_version": response.webhook_details.webhook_version
        },
        "enable_payment_response_hash": response.enable_payment_response_hash,
        "redirect_to_merchant_with_http_post": response.redirect_to_merchant_with_http_post,
        "metadata": response.metadata
    }

@mcp.tool("Update a Profile")
def update_profile(
    merchant_id: str,
    profile_id: str,
    profile_name: str,
    return_url: str = "https://example.com/return",
    webhook_url: str = "https://example.com/webhook",
    webhook_version: str = "1.0.1",
    enable_payment_response_hash: bool = True,
    redirect_to_merchant_with_http_post: bool = False
) -> Dict[str, Any]:
    """
    Update an existing profile.
    
    Args:
        merchant_id: The ID of the merchant
        profile_id: The ID of the profile to update
        profile_name: New name for the profile
        return_url: New return URL
        webhook_url: New webhook URL
        webhook_version: New webhook version
        enable_payment_response_hash: Whether to enable payment response hash
        redirect_to_merchant_with_http_post: Whether to redirect to merchant with HTTP POST
        
    Returns:
        A dictionary containing the updated profile details
    """
    client = get_api_key_auth()
    profile_api = ProfileApi(client)
    
    webhook_details = WebhookDetails(
        webhook_url=webhook_url,
        webhook_version=webhook_version,
        webhook_username="test_username",
        webhook_password="test_password",
        payment_created_enabled=True,
        payment_succeeded_enabled=True,
        payment_failed_enabled=True
    )
    
    update_request = ProfileCreate(
        profile_name=profile_name,
        return_url=return_url,
        enable_payment_response_hash=enable_payment_response_hash,
        redirect_to_merchant_with_http_post=redirect_to_merchant_with_http_post,
        webhook_details=webhook_details
    )
    
    response = profile_api.update_a_profile(merchant_id, profile_id, update_request)
    return {
        "profile_id": response.profile_id,
        "profile_name": response.profile_name,
        "return_url": response.return_url,
        "webhook_details": {
            "webhook_url": response.webhook_details.webhook_url,
            "webhook_version": response.webhook_details.webhook_version
        },
        "enable_payment_response_hash": response.enable_payment_response_hash,
        "redirect_to_merchant_with_http_post": response.redirect_to_merchant_with_http_post,
        "metadata": response.metadata
    }

@mcp.tool("List Profiles")
def list_profiles(merchant_id: str, user_info_token: str) -> Dict[str, Any]:
    """
    List all profiles for a merchant using a user JWT token.
    
    Args:
        merchant_id: The ID of the merchant
        user_info_token: The user JWT token for authentication
        
    Returns:
        A dictionary containing the list of profiles
    """
    # Configure API client with JWT Bearer token
    config = Configuration(
        host="http://localhost:8080",
        api_key={"Authorization": f"Bearer {user_info_token}"}
    )
    client = ApiClient(configuration=config)
    profile_api = ProfileApi(client)
    
    try: # Add try-except block for better error handling
        response = profile_api.list_profiles(merchant_id)
        profiles = []
        if response: # Check if response is not None
            for profile in response:
                profile_data = {
                    "profile_id": profile.profile_id,
                    "profile_name": profile.profile_name,
                    "return_url": profile.return_url,
                    "enable_payment_response_hash": profile.enable_payment_response_hash,
                    "redirect_to_merchant_with_http_post": profile.redirect_to_merchant_with_http_post,
                    "metadata": profile.metadata
                }
                # Safely access webhook details if they exist
                if profile.webhook_details:
                    profile_data["webhook_details"] = {
                        "webhook_url": profile.webhook_details.webhook_url,
                        "webhook_version": profile.webhook_details.webhook_version
                    }
                else:
                    profile_data["webhook_details"] = None
                profiles.append(profile_data)
        return {"profiles": profiles, "success": True}
    except Exception as e:
        # Log the error or handle it appropriately
        print(f"Error in list_profiles: {e}") 
        # Attempt to extract details if it's an ApiException
        error_body = None
        status_code = None
        if hasattr(e, 'status'):
            status_code = e.status
        if hasattr(e, 'body'):
            error_body = e.body
        return {"error": str(e), "error_body": error_body, "status_code": status_code, "success": False}

@mcp.tool("Create a Customer")
def create_customer(
    merchant_id: str,
    email: str,
    name: str,
    phone: str,
    description: str = "Customer created via MCP"
) -> Dict[str, Any]:
    """
    Create a new customer.
    
    Args:
        merchant_id: The ID of the merchant
        email: Customer's email address
        name: Customer's name
        phone: Customer's phone number
        description: Description of the customer
        
    Returns:
        A dictionary containing the customer details
    """
    client = get_merchant_api_key_auth(merchant_id)
    customers_api = CustomersApi(client)
    
    customer_request = CustomerRequest(
        email=email,
        name=name,
        phone=phone,
        description=description,
        metadata={
            "created_via": "mcp"
        }
    )
    
    response = customers_api.create_a_customer(customer_request)
    return {
        "customer_id": response.customer_id,
        "email": response.email,
        "name": response.name,
        "phone": response.phone,
        "description": response.description,
        "metadata": response.metadata
    }

@mcp.tool("Retrieve a Customer")
def retrieve_customer(
    merchant_id: str,
    customer_id: str
) -> Dict[str, Any]:
    """
    Retrieve details of an existing customer.
    
    Args:
        merchant_id: The ID of the merchant
        customer_id: The ID of the customer to retrieve
        
    Returns:
        A dictionary containing the customer details
    """
    client = get_merchant_api_key_auth(merchant_id)
    customers_api = CustomersApi(client)
    
    response = customers_api.retrieve_a_customer(customer_id)
    return {
        "customer_id": response.customer_id,
        "email": response.email,
        "name": response.name,
        "phone": response.phone,
        "description": response.description,
        "metadata": response.metadata
    }

@mcp.tool("Update a Customer")
def update_customer(
    merchant_id: str,
    customer_id: str,
    email: str,
    name: str,
    phone: str,
    description: str = "Customer updated via MCP"
) -> Dict[str, Any]:
    """
    Update an existing customer.
    
    Args:
        merchant_id: The ID of the merchant
        customer_id: The ID of the customer to update
        email: New email address
        name: New name
        phone: New phone number
        description: New description
        
    Returns:
        A dictionary containing the updated customer details
    """
    client = get_merchant_api_key_auth(merchant_id)
    customers_api = CustomersApi(client)
    
    update_request = CustomerUpdateRequest(
        email=email,
        name=name,
        phone=phone,
        description=description,
        metadata={
            "updated_via": "mcp",
            "updated_at": "now"
        }
    )
    
    response = customers_api.update_a_customer(customer_id, update_request)
    return {
        "customer_id": response.customer_id,
        "email": response.email,
        "name": response.name,
        "phone": response.phone,
        "description": response.description,
        "metadata": response.metadata
    }

@mcp.tool("Delete a Customer")
def delete_customer(
    merchant_id: str,
    customer_id: str
) -> Dict[str, Any]:
    """
    Delete an existing customer.
    
    Args:
        merchant_id: The ID of the merchant
        customer_id: The ID of the customer to delete
        
    Returns:
        A dictionary containing the deletion status
    """
    client = get_merchant_api_key_auth(merchant_id)
    customers_api = CustomersApi(client)
    
    response = customers_api.delete_a_customer(customer_id)
    return {
        "customer_id": response.customer_id,
        "customer_deleted": response.customer_deleted,
        "address_deleted": response.address_deleted,
        "payment_methods_deleted": response.payment_methods_deleted
    }

@mcp.tool("Create a Merchant Connector")
def create_merchant_connector(
    merchant_id: str,
    connector_type: str = "payment_processor",
    connector_name: str = "stripe",
    auth_type: str = "HeaderKey",
    api_key: str = "test_api_key",
    profile_id: str = None,
    test_mode: bool = False,
    disabled: bool = False,
    business_country: str = "US",
    business_label: str = "default",
    metadata: Dict[str, Any] = None
) -> Dict[str, Any]:
    """
    Create a new merchant connector.
    
    Args:
        merchant_id: The ID of the merchant
        connector_type: Type of connector (default: payment_processor)
        connector_name: Name of the connector (default: stripe)
        auth_type: Authentication type (default: HeaderKey)
        api_key: API key for the connector
        profile_id: Profile identifier
        test_mode: Whether to enable test mode
        disabled: Whether to disable the connector
        business_country: Country code for business
        business_label: Business label
        metadata: Additional metadata
        
    Returns:
        A dictionary containing the merchant connector details
    """
    client = get_api_key_auth()
    connector_api = MerchantConnectorAccountApi(client)
    
    # Create connector details
    merchant_connector_details = MerchantConnectorDetails(
        auth_type=auth_type,
        api_key=api_key
    )
    
    # Define payment methods
    payment_methods_enabled = [
        {
            "payment_method": "card",
            "payment_method_types": [
                {
                    "payment_method_type": "credit",
                    "card_networks": ["Visa", "Mastercard", "DinersClub", "Discover"],
                    "minimum_amount": -1,
                    "maximum_amount": 68607706,
                    "recurring_enabled": True,
                    "installment_payment_enabled": True
                }
            ]
        }
    ]
    
    # Create dynamic connector label with timestamp
    from datetime import datetime
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    connector_label = f"test_connector_{timestamp}"
    
    connector_request = MerchantConnectorCreate(
        connector_type=connector_type,
        connector_name=connector_name,
        connector_account_details=merchant_connector_details,
        connector_label=connector_label,
        profile_id=profile_id,
        payment_methods_enabled=payment_methods_enabled,
        test_mode=test_mode,
        disabled=disabled,
        business_country=business_country,
        business_label=business_label,
        metadata=metadata or {}
    )
    
    response = connector_api.create_a_merchant_connector(
        account_id=merchant_id,
        merchant_connector_create=connector_request
    )
    
    return {
        "merchant_connector_id": response.merchant_connector_id,
        "connector_type": response.connector_type,
        "connector_name": response.connector_name,
        "test_mode": response.test_mode,
        "status": response.status
    }

@mcp.tool("Retrieve a Merchant Connector")
def retrieve_merchant_connector(
    merchant_id: str,
    connector_id: str
) -> Dict[str, Any]:
    """
    Retrieve details of a merchant connector.
    
    Args:
        merchant_id: The ID of the merchant
        connector_id: The ID of the connector to retrieve
        
    Returns:
        A dictionary containing the merchant connector details
    """
    client = get_api_key_auth()
    connector_api = MerchantConnectorAccountApi(client)
    
    response = connector_api.retrieve_a_merchant_connector(
        account_id=merchant_id,
        connector_id=connector_id
    )
    
    return {
        "merchant_connector_id": response.merchant_connector_id,
        "connector_type": response.connector_type,
        "connector_name": response.connector_name,
        "connector_label": response.connector_label,
        "payment_methods_enabled": response.payment_methods_enabled,
        "test_mode": response.test_mode,
        "disabled": response.disabled,
        "status": response.status,
        "metadata": response.metadata
    }

@mcp.tool("Update a Merchant Connector")
def update_merchant_connector(
    merchant_id: str,
    connector_id: str,
    connector_type: str,
    connector_name: str,
    payment_methods_enabled: List[Dict[str, Any]] = None,
    test_mode: bool = False,
    status: str = "active",
    metadata: Dict[str, Any] = None
) -> Dict[str, Any]:
    """
    Update an existing merchant connector.
    
    Args:
        merchant_id: The ID of the merchant
        connector_id: The ID of the connector to update
        connector_type: Type of connector
        connector_name: Name of the connector
        payment_methods_enabled: List of enabled payment methods
        test_mode: Whether to enable test mode
        status: Status of the connector (active/inactive)
        metadata: Additional metadata
        
    Returns:
        A dictionary containing the updated merchant connector details
    """
    client = get_api_key_auth()
    connector_api = MerchantConnectorAccountApi(client)
    
    # Get current connector data to preserve existing values
    current_connector = connector_api.retrieve_a_merchant_connector(
        account_id=merchant_id,
        connector_id=connector_id
    )
    
    update_request = MerchantConnectorUpdate(
        connector_type=connector_type,
        connector_name=connector_name,
        payment_methods_enabled=payment_methods_enabled or current_connector.payment_methods_enabled,
        test_mode=test_mode,
        status=status,
        metadata=metadata or {}
    )
    
    response = connector_api.update_a_merchant_connector(
        account_id=merchant_id,
        connector_id=connector_id,
        merchant_connector_update=update_request
    )
    
    return {
        "merchant_connector_id": response.merchant_connector_id,
        "connector_type": response.connector_type,
        "connector_name": response.connector_name,
        "payment_methods_enabled": response.payment_methods_enabled,
        "test_mode": response.test_mode,
        "status": response.status,
        "metadata": response.metadata
    }

@mcp.tool("Delete a Merchant Connector")
def delete_merchant_connector(
    merchant_id: str,
    connector_id: str
) -> Dict[str, Any]:
    """
    Delete an existing merchant connector.
    
    Args:
        merchant_id: The ID of the merchant
        connector_id: The ID of the connector to delete
        
    Returns:
        A dictionary containing the deletion status
    """
    client = get_api_key_auth()
    connector_api = MerchantConnectorAccountApi(client)
    
    response = connector_api.delete_a_merchant_connector(account_id=merchant_id, connector_id=int(connector_id))
    return {
        "status": "success",
        "message": "Merchant connector deleted successfully"
    }

@mcp.tool("Create a Single Routing Algorithm")
def create_single_routing_algorithm(
    merchant_id: str,
    profile_id: str,
    name: str,
    description: str,
    connector_name: str,
    connector_type: str = "payment_processor"
) -> Dict:
    """
    Create a single connector routing algorithm.
    
    Args:
        merchant_id: The ID of the merchant
        profile_id: The ID of the profile
        name: Name for the routing algorithm
        description: Description of the routing algorithm
        connector_name: Name of the connector to route to
        connector_type: Type of the connector (default: payment_processor)
        
    Returns:
        A dictionary containing the routing algorithm details
    """
    client = get_api_key_auth()
    api_instance = RoutingApi(client)
    
    # Create the connector choice
    connector = RoutableConnectorChoice(
        connector=connector_name,
        merchant_connector_id=None
    )
    
    # Create the algorithm
    algorithm = RoutingAlgorithm(
        type="single",
        data=connector
    )
    
    # Create the request
    request = RoutingConfigRequest(
        name=name,
        description=description,
        algorithm=algorithm,
        profile_id=profile_id
    )
    
    try:
        response = api_instance.create_a_routing_config(request)
        return response.to_dict()
    except Exception as e:
        raise Exception(f"Error creating routing algorithm: {str(e)}")

@mcp.tool("Create a Priority Routing Algorithm")
def create_priority_routing_algorithm(
    merchant_id: str,
    profile_id: str,
    name: str,
    description: str,
    connectors: List[Dict[str, str]]
) -> Dict:
    """
    Create a priority-based routing algorithm.
    
    Args:
        merchant_id: The ID of the merchant
        profile_id: The ID of the profile
        name: Name for the routing algorithm
        description: Description of the routing algorithm
        connectors: List of connector configurations in priority order
            Each connector should have 'name' and 'type' (default: payment_processor)
        
    Returns:
        A dictionary containing the routing algorithm details
    """
    client = get_api_key_auth()
    api_instance = RoutingApi(client)
    
    # Create the connector choices
    connector_choices = []
    for conn in connectors:
        connector = RoutableConnectorChoice(
            connector=conn['name'],
            merchant_connector_id=None
        )
        connector_choices.append(connector)
    
    # Create the algorithm
    algorithm = RoutingAlgorithm(
        type="priority",
        data=connector_choices
    )
    
    # Create the request
    request = RoutingConfigRequest(
        name=name,
        description=description,
        algorithm=algorithm,
        profile_id=profile_id
    )
    
    try:
        response = api_instance.create_a_routing_config(request)
        return response.to_dict()
    except Exception as e:
        raise Exception(f"Error creating routing algorithm: {str(e)}")

@mcp.tool("Create a Volume Split Routing Algorithm")
def create_volume_split_routing_algorithm(
    merchant_id: str,
    profile_id: str,
    name: str,
    description: str,
    splits: List[Dict]
) -> Dict:
    """
    Create a volume split routing algorithm.
    
    Args:
        merchant_id: The ID of the merchant
        profile_id: The ID of the profile
        name: Name for the routing algorithm
        description: Description of the routing algorithm
        splits: List of connector splits
            Each split should have:
            - 'connector_name': Name of the connector
            - 'connector_type': Type of the connector (default: payment_processor)
            - 'split': Percentage of volume (0-100)
        
    Returns:
        A dictionary containing the routing algorithm details
    """
    client = get_api_key_auth()
    api_instance = RoutingApi(client)
    
    # Create the volume splits
    volume_splits = []
    for split in splits:
        connector = RoutableConnectorChoice(
            connector=split['connector_name'],
            merchant_connector_id=None
        )
        volume_split = ConnectorVolumeSplit(
            connector=connector,
            split=split['split']
        )
        volume_splits.append(volume_split)
    
    # Create the algorithm
    algorithm = RoutingAlgorithm(
        type="volume_split",
        data=volume_splits
    )
    
    # Create the request
    request = RoutingConfigRequest(
        name=name,
        description=description,
        algorithm=algorithm,
        profile_id=profile_id
    )
    
    try:
        response = api_instance.create_a_routing_config(request)
        return response.to_dict()
    except Exception as e:
        raise Exception(f"Error creating routing algorithm: {str(e)}")

@mcp.tool("Retrieve a Routing Algorithm")
def retrieve_routing_algorithm(
    merchant_id: str,
    profile_id: str,
    algorithm_id: str
) -> Dict:
    """
    Retrieve details of a routing algorithm.
    
    Args:
        merchant_id: The ID of the merchant
        profile_id: The ID of the profile
        algorithm_id: The ID of the routing algorithm to retrieve
        
    Returns:
        A dictionary containing the routing algorithm details
    """
    client = get_api_key_auth()
    api_instance = RoutingApi(client)
    
    try:
        response = api_instance.retrieve_a_routing_config(algorithm_id)
        return response.to_dict()
    except Exception as e:
        raise Exception(f"Error retrieving routing algorithm: {str(e)}")

@mcp.tool("List Routing Algorithms")
def list_routing_algorithms(
    merchant_id: str,
    profile_id: str,
    limit: Optional[int] = None,
    offset: Optional[int] = None
) -> Dict:
    """
    List all routing algorithms for a profile.
    
    Args:
        merchant_id: The ID of the merchant
        profile_id: The ID of the profile
        limit: Maximum number of records to return
        offset: Number of records to skip
        
    Returns:
        A dictionary containing the list of routing algorithms
    """
    client = get_api_key_auth()
    api_instance = RoutingApi(client)
    
    try:
        response = api_instance.list_routing_configs(
            profile_id=profile_id,
            limit=limit,
            offset=offset
        )
        return response.to_dict()
    except Exception as e:
        raise Exception(f"Error listing routing algorithms: {str(e)}")

@mcp.tool("Delete a Routing Algorithm")
def delete_routing_algorithm(
    merchant_id: str,
    profile_id: str,
    algorithm_id: str
) -> Dict:
    """
    Delete a routing algorithm.
    
    Args:
        merchant_id: The ID of the merchant
        profile_id: The ID of the profile
        algorithm_id: The ID of the routing algorithm to delete
        
    Returns:
        A dictionary containing the deletion status
    """
    client = get_api_key_auth()
    api_instance = RoutingApi(client)
    
    try:
        # Note: The actual deletion endpoint is not available in the API
        # This is a placeholder for when the endpoint becomes available
        raise NotImplementedError("Deletion of routing algorithms is not yet supported")
    except Exception as e:
        raise Exception(f"Error deleting routing algorithm: {str(e)}")

if __name__ == "__main__":
    mcp.run(transport="stdio")