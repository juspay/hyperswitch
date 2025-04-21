# hyperswitch.ProfileApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_a_profile**](ProfileApi.md#create_a_profile) | **POST** /account/{account_id}/business_profile | Profile - Create
[**delete_the_profile**](ProfileApi.md#delete_the_profile) | **DELETE** /account/{account_id}/business_profile/{profile_id} | Profile - Delete
[**list_profiles**](ProfileApi.md#list_profiles) | **GET** /account/{account_id}/business_profile | Profile - List
[**retrieve_a_profile**](ProfileApi.md#retrieve_a_profile) | **GET** /account/{account_id}/business_profile/{profile_id} | Profile - Retrieve
[**update_a_profile**](ProfileApi.md#update_a_profile) | **POST** /account/{account_id}/business_profile/{profile_id} | Profile - Update


# **create_a_profile**
> ProfileResponse create_a_profile(account_id, profile_create)

Profile - Create

Creates a new *profile* for a merchant

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.profile_create import ProfileCreate
from hyperswitch.models.profile_response import ProfileResponse
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

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.ProfileApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    profile_create = {} # ProfileCreate | 

    try:
        # Profile - Create
        api_response = api_instance.create_a_profile(account_id, profile_create)
        print("The response of ProfileApi->create_a_profile:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling ProfileApi->create_a_profile: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **profile_create** | [**ProfileCreate**](ProfileCreate.md)|  | 

### Return type

[**ProfileResponse**](ProfileResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Profile Created |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **delete_the_profile**
> bool delete_the_profile(account_id, profile_id)

Profile - Delete

Delete the *profile*

### Example

* Api Key Authentication (admin_api_key):

```python
import hyperswitch
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
    api_instance = hyperswitch.ProfileApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    profile_id = 'profile_id_example' # str | The unique identifier for the profile

    try:
        # Profile - Delete
        api_response = api_instance.delete_the_profile(account_id, profile_id)
        print("The response of ProfileApi->delete_the_profile:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling ProfileApi->delete_the_profile: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **profile_id** | **str**| The unique identifier for the profile | 

### Return type

**bool**

### Authorization

[admin_api_key](../README.md#admin_api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: text/plain

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Profiles Deleted |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_profiles**
> List[ProfileResponse] list_profiles(account_id)

Profile - List

Lists all the *profiles* under a merchant

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.profile_response import ProfileResponse
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

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.ProfileApi(api_client)
    account_id = 'account_id_example' # str | Merchant Identifier

    try:
        # Profile - List
        api_response = api_instance.list_profiles(account_id)
        print("The response of ProfileApi->list_profiles:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling ProfileApi->list_profiles: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| Merchant Identifier | 

### Return type

[**List[ProfileResponse]**](ProfileResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Profiles Retrieved |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_profile**
> ProfileResponse retrieve_a_profile(account_id, profile_id)

Profile - Retrieve

Retrieve existing *profile*

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.profile_response import ProfileResponse
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

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.ProfileApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    profile_id = 'profile_id_example' # str | The unique identifier for the profile

    try:
        # Profile - Retrieve
        api_response = api_instance.retrieve_a_profile(account_id, profile_id)
        print("The response of ProfileApi->retrieve_a_profile:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling ProfileApi->retrieve_a_profile: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **profile_id** | **str**| The unique identifier for the profile | 

### Return type

[**ProfileResponse**](ProfileResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Profile Updated |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_a_profile**
> ProfileResponse update_a_profile(account_id, profile_id, profile_create)

Profile - Update

Update the *profile*

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.profile_create import ProfileCreate
from hyperswitch.models.profile_response import ProfileResponse
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

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.ProfileApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    profile_id = 'profile_id_example' # str | The unique identifier for the profile
    profile_create = {"profile_name":"shoe_business"} # ProfileCreate | 

    try:
        # Profile - Update
        api_response = api_instance.update_a_profile(account_id, profile_id, profile_create)
        print("The response of ProfileApi->update_a_profile:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling ProfileApi->update_a_profile: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **profile_id** | **str**| The unique identifier for the profile | 
 **profile_create** | [**ProfileCreate**](ProfileCreate.md)|  | 

### Return type

[**ProfileResponse**](ProfileResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Profile Updated |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

