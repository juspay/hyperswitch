# hyperswitch.CustomersApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_a_customer**](CustomersApi.md#create_a_customer) | **POST** /customers | Customers - Create
[**delete_a_customer**](CustomersApi.md#delete_a_customer) | **DELETE** /customers/{customer_id} | Customers - Delete
[**list_all_customers_for_a_merchant**](CustomersApi.md#list_all_customers_for_a_merchant) | **GET** /customers/list | Customers - List
[**retrieve_a_customer**](CustomersApi.md#retrieve_a_customer) | **GET** /customers/{customer_id} | Customers - Retrieve
[**update_a_customer**](CustomersApi.md#update_a_customer) | **POST** /customers/{customer_id} | Customers - Update


# **create_a_customer**
> CustomerResponse create_a_customer(customer_request)

Customers - Create

Creates a customer object and stores the customer details to be reused for future payments.
Incase the customer already exists in the system, this API will respond with the customer details.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.customer_request import CustomerRequest
from hyperswitch.models.customer_response import CustomerResponse
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
    api_instance = hyperswitch.CustomersApi(api_client)
    customer_request = {"email":"guest@example.com","name":"John Doe"} # CustomerRequest | 

    try:
        # Customers - Create
        api_response = api_instance.create_a_customer(customer_request)
        print("The response of CustomersApi->create_a_customer:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CustomersApi->create_a_customer: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **customer_request** | [**CustomerRequest**](CustomerRequest.md)|  | 

### Return type

[**CustomerResponse**](CustomerResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Customer Created |  -  |
**400** | Invalid data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **delete_a_customer**
> CustomerDeleteResponse delete_a_customer(customer_id)

Customers - Delete

Delete a customer record.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.customer_delete_response import CustomerDeleteResponse
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
    api_instance = hyperswitch.CustomersApi(api_client)
    customer_id = 'customer_id_example' # str | The unique identifier for the Customer

    try:
        # Customers - Delete
        api_response = api_instance.delete_a_customer(customer_id)
        print("The response of CustomersApi->delete_a_customer:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CustomersApi->delete_a_customer: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **customer_id** | **str**| The unique identifier for the Customer | 

### Return type

[**CustomerDeleteResponse**](CustomerDeleteResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Customer was Deleted |  -  |
**404** | Customer was not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_customers_for_a_merchant**
> List[CustomerResponse] list_all_customers_for_a_merchant()

Customers - List

Lists all the customers for a particular merchant id.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.customer_response import CustomerResponse
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
    api_instance = hyperswitch.CustomersApi(api_client)

    try:
        # Customers - List
        api_response = api_instance.list_all_customers_for_a_merchant()
        print("The response of CustomersApi->list_all_customers_for_a_merchant:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CustomersApi->list_all_customers_for_a_merchant: %s\n" % e)
```



### Parameters

This endpoint does not need any parameter.

### Return type

[**List[CustomerResponse]**](CustomerResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Customers retrieved |  -  |
**400** | Invalid Data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_customer**
> CustomerResponse retrieve_a_customer(customer_id)

Customers - Retrieve

Retrieves a customer's details.

### Example

* Api Key Authentication (api_key):
* Api Key Authentication (ephemeral_key):

```python
import hyperswitch
from hyperswitch.models.customer_response import CustomerResponse
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

# Configure API key authorization: ephemeral_key
configuration.api_key['ephemeral_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['ephemeral_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.CustomersApi(api_client)
    customer_id = 'customer_id_example' # str | The unique identifier for the Customer

    try:
        # Customers - Retrieve
        api_response = api_instance.retrieve_a_customer(customer_id)
        print("The response of CustomersApi->retrieve_a_customer:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CustomersApi->retrieve_a_customer: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **customer_id** | **str**| The unique identifier for the Customer | 

### Return type

[**CustomerResponse**](CustomerResponse.md)

### Authorization

[api_key](../README.md#api_key), [ephemeral_key](../README.md#ephemeral_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Customer Retrieved |  -  |
**404** | Customer was not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_a_customer**
> CustomerResponse update_a_customer(customer_id, customer_update_request)

Customers - Update

Updates the customer's details in a customer object.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.customer_response import CustomerResponse
from hyperswitch.models.customer_update_request import CustomerUpdateRequest
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
    api_instance = hyperswitch.CustomersApi(api_client)
    customer_id = 'customer_id_example' # str | The unique identifier for the Customer
    customer_update_request = {"email":"guest@example.com","name":"John Doe"} # CustomerUpdateRequest | 

    try:
        # Customers - Update
        api_response = api_instance.update_a_customer(customer_id, customer_update_request)
        print("The response of CustomersApi->update_a_customer:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CustomersApi->update_a_customer: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **customer_id** | **str**| The unique identifier for the Customer | 
 **customer_update_request** | [**CustomerUpdateRequest**](CustomerUpdateRequest.md)|  | 

### Return type

[**CustomerResponse**](CustomerResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Customer was Updated |  -  |
**404** | Customer was not found |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

