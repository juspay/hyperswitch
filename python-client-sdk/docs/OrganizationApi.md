# hyperswitch.OrganizationApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_an_organization**](OrganizationApi.md#create_an_organization) | **POST** /organization | Organization - Create
[**retrieve_an_organization**](OrganizationApi.md#retrieve_an_organization) | **GET** /organization/{id} | Organization - Retrieve
[**update_an_organization**](OrganizationApi.md#update_an_organization) | **PUT** /organization/{id} | Organization - Update


# **create_an_organization**
> OrganizationResponse create_an_organization(organization_create_request)

Organization - Create

Create a new organization

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.organization_create_request import OrganizationCreateRequest
from hyperswitch.models.organization_response import OrganizationResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: admin_api_key
configuration.api_key['admin_api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['admin_api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.OrganizationApi(api_client)
    organization_create_request = {"organization_name":"organization_abc"} # OrganizationCreateRequest | 

    try:
        # Organization - Create
        api_response = api_instance.create_an_organization(organization_create_request)
        print("The response of OrganizationApi->create_an_organization:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling OrganizationApi->create_an_organization: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **organization_create_request** | [**OrganizationCreateRequest**](OrganizationCreateRequest.md)|  | 

### Return type

[**OrganizationResponse**](OrganizationResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Organization Created |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_an_organization**
> OrganizationResponse retrieve_an_organization(id)

Organization - Retrieve

Retrieve an existing organization

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.organization_response import OrganizationResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: admin_api_key
configuration.api_key['admin_api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['admin_api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.OrganizationApi(api_client)
    id = 'id_example' # str | The unique identifier for the Organization

    try:
        # Organization - Retrieve
        api_response = api_instance.retrieve_an_organization(id)
        print("The response of OrganizationApi->retrieve_an_organization:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling OrganizationApi->retrieve_an_organization: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **str**| The unique identifier for the Organization | 

### Return type

[**OrganizationResponse**](OrganizationResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Organization Created |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_an_organization**
> OrganizationResponse update_an_organization(id, organization_update_request)

Organization - Update

Create a new organization for .

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
from hyperswitch.models.organization_response import OrganizationResponse
from hyperswitch.models.organization_update_request import OrganizationUpdateRequest
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)

# The client must configure the authentication and authorization parameters
# in accordance with the API server security policy.
# Examples for each auth method are provided below, use the example that
# satisfies your auth use case.

# Configure API key authorization: admin_api_key
configuration.api_key['admin_api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['admin_api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.OrganizationApi(api_client)
    id = 'id_example' # str | The unique identifier for the Organization
    organization_update_request = {"organization_name":"organization_abcd"} # OrganizationUpdateRequest | 

    try:
        # Organization - Update
        api_response = api_instance.update_an_organization(id, organization_update_request)
        print("The response of OrganizationApi->update_an_organization:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling OrganizationApi->update_an_organization: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **id** | **str**| The unique identifier for the Organization | 
 **organization_update_request** | [**OrganizationUpdateRequest**](OrganizationUpdateRequest.md)|  | 

### Return type

[**OrganizationResponse**](OrganizationResponse.md)

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Organization Created |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

