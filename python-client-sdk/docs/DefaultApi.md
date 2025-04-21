# hyperswitch.DefaultApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**accounts_account_id_connectors_get**](DefaultApi.md#accounts_account_id_connectors_get) | **GET** /accounts/{account_id}/connectors | 
[**customers_customer_id_mandates_get**](DefaultApi.md#customers_customer_id_mandates_get) | **GET** /customers/{customer_id}/mandates | 
[**list_customer_payment_methods**](DefaultApi.md#list_customer_payment_methods) | **GET** /customers/payment_methods | 
[**payments_payment_id_post_session_tokens_post**](DefaultApi.md#payments_payment_id_post_session_tokens_post) | **POST** /payments/{payment_id}/post_session_tokens | 
[**payments_payment_id_update_metadata_post**](DefaultApi.md#payments_payment_id_update_metadata_post) | **POST** /payments/{payment_id}/update_metadata | 
[**relay_relay_id_get**](DefaultApi.md#relay_relay_id_get) | **GET** /relay/{relay_id} | 


# **accounts_account_id_connectors_get**
> List[Connector] accounts_account_id_connectors_get(account_id)

### Example


```python
import hyperswitch
from hyperswitch.models.connector import Connector
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)


# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.DefaultApi(api_client)
    account_id = 'account_id_example' # str | The identifier for account

    try:
        api_response = api_instance.accounts_account_id_connectors_get(account_id)
        print("The response of DefaultApi->accounts_account_id_connectors_get:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->accounts_account_id_connectors_get: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The identifier for account | 

### Return type

[**List[Connector]**](Connector.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | List of account connectors |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **customers_customer_id_mandates_get**
> List[Mandate] customers_customer_id_mandates_get(customer_id)

### Example


```python
import hyperswitch
from hyperswitch.models.mandate import Mandate
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)


# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.DefaultApi(api_client)
    customer_id = 'customer_id_example' # str | The identifier for customer

    try:
        api_response = api_instance.customers_customer_id_mandates_get(customer_id)
        print("The response of DefaultApi->customers_customer_id_mandates_get:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->customers_customer_id_mandates_get: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **customer_id** | **str**| The identifier for customer | 

### Return type

[**List[Mandate]**](Mandate.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | List of customer mandates |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_customer_payment_methods**
> List[PaymentMethod] list_customer_payment_methods()

### Example


```python
import hyperswitch
from hyperswitch.models.payment_method import PaymentMethod
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)


# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.DefaultApi(api_client)

    try:
        api_response = api_instance.list_customer_payment_methods()
        print("The response of DefaultApi->list_customer_payment_methods:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->list_customer_payment_methods: %s\n" % e)
```



### Parameters

This endpoint does not need any parameter.

### Return type

[**List[PaymentMethod]**](PaymentMethod.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | List of customer payment methods |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **payments_payment_id_post_session_tokens_post**
> PaymentsPaymentIdPostSessionTokensPost200Response payments_payment_id_post_session_tokens_post(payment_id)

### Example


```python
import hyperswitch
from hyperswitch.models.payments_payment_id_post_session_tokens_post200_response import PaymentsPaymentIdPostSessionTokensPost200Response
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)


# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.DefaultApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment

    try:
        api_response = api_instance.payments_payment_id_post_session_tokens_post(payment_id)
        print("The response of DefaultApi->payments_payment_id_post_session_tokens_post:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->payments_payment_id_post_session_tokens_post: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 

### Return type

[**PaymentsPaymentIdPostSessionTokensPost200Response**](PaymentsPaymentIdPostSessionTokensPost200Response.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Session tokens response |  -  |
**400** | Bad Request |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **payments_payment_id_update_metadata_post**
> PaymentMetadataResponse payments_payment_id_update_metadata_post(payment_id)

### Example


```python
import hyperswitch
from hyperswitch.models.payment_metadata_response import PaymentMetadataResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)


# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.DefaultApi(api_client)
    payment_id = 'payment_id_example' # str | The identifier for payment

    try:
        api_response = api_instance.payments_payment_id_update_metadata_post(payment_id)
        print("The response of DefaultApi->payments_payment_id_update_metadata_post:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->payments_payment_id_update_metadata_post: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_id** | **str**| The identifier for payment | 

### Return type

[**PaymentMetadataResponse**](PaymentMetadataResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Metadata update response |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **relay_relay_id_get**
> RelayResponse relay_relay_id_get(relay_id)

### Example


```python
import hyperswitch
from hyperswitch.models.relay_response import RelayResponse
from hyperswitch.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to https://sandbox.hyperswitch.io
# See configuration.py for a list of all supported configuration parameters.
configuration = hyperswitch.Configuration(
    host = "https://sandbox.hyperswitch.io"
)


# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.DefaultApi(api_client)
    relay_id = 'relay_id_example' # str | The identifier for relay

    try:
        api_response = api_instance.relay_relay_id_get(relay_id)
        print("The response of DefaultApi->relay_relay_id_get:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling DefaultApi->relay_relay_id_get: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **relay_id** | **str**| The identifier for relay | 

### Return type

[**RelayResponse**](RelayResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Relay response |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

