# hyperswitch.PaymentMethodsApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_a_payment_method**](PaymentMethodsApi.md#create_a_payment_method) | **POST** /payment_methods | PaymentMethods - Create
[**delete_a_payment_method**](PaymentMethodsApi.md#delete_a_payment_method) | **DELETE** /payment_methods/{method_id} | Payment Method - Delete
[**list_all_payment_methods_for_a_customer**](PaymentMethodsApi.md#list_all_payment_methods_for_a_customer) | **GET** /customers/{customer_id}/payment_methods | List payment methods for a Customer
[**list_all_payment_methods_for_a_customer_0**](PaymentMethodsApi.md#list_all_payment_methods_for_a_customer_0) | **GET** /customers/payment_methods | List customer saved payment methods for a Payment
[**list_all_payment_methods_for_a_merchant**](PaymentMethodsApi.md#list_all_payment_methods_for_a_merchant) | **GET** /account/payment_methods | List payment methods for a Merchant
[**retrieve_a_payment_method**](PaymentMethodsApi.md#retrieve_a_payment_method) | **GET** /payment_methods/{method_id} | Payment Method - Retrieve
[**set_the_payment_method_as_default**](PaymentMethodsApi.md#set_the_payment_method_as_default) | **GET** /{customer_id}/payment_methods/{payment_method_id}/default | Payment Method - Set Default Payment Method for Customer
[**update_a_payment_method**](PaymentMethodsApi.md#update_a_payment_method) | **POST** /payment_methods/{method_id}/update | Payment Method - Update


# **create_a_payment_method**
> PaymentMethodResponse create_a_payment_method(payment_method_create)

PaymentMethods - Create

Creates and stores a payment method against a customer.
In case of cards, this API should be used only by PCI compliant merchants.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payment_method_create import PaymentMethodCreate
from hyperswitch.models.payment_method_response import PaymentMethodResponse
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
    api_instance = hyperswitch.PaymentMethodsApi(api_client)
    payment_method_create = {"card":{"card_exp_month":"11","card_exp_year":"25","card_holder_name":"John Doe","card_number":"4242424242424242"},"customer_id":"{{customer_id}}","payment_method":"card","payment_method_issuer":"Visa","payment_method_type":"credit"} # PaymentMethodCreate | 

    try:
        # PaymentMethods - Create
        api_response = api_instance.create_a_payment_method(payment_method_create)
        print("The response of PaymentMethodsApi->create_a_payment_method:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentMethodsApi->create_a_payment_method: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **payment_method_create** | [**PaymentMethodCreate**](PaymentMethodCreate.md)|  | 

### Return type

[**PaymentMethodResponse**](PaymentMethodResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment Method Created |  -  |
**400** | Invalid Data |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **delete_a_payment_method**
> PaymentMethodDeleteResponse delete_a_payment_method(method_id)

Payment Method - Delete

Deletes a payment method of a customer.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payment_method_delete_response import PaymentMethodDeleteResponse
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
    api_instance = hyperswitch.PaymentMethodsApi(api_client)
    method_id = 'method_id_example' # str | The unique identifier for the Payment Method

    try:
        # Payment Method - Delete
        api_response = api_instance.delete_a_payment_method(method_id)
        print("The response of PaymentMethodsApi->delete_a_payment_method:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentMethodsApi->delete_a_payment_method: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **method_id** | **str**| The unique identifier for the Payment Method | 

### Return type

[**PaymentMethodDeleteResponse**](PaymentMethodDeleteResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment Method deleted |  -  |
**404** | Payment Method does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_payment_methods_for_a_customer**
> CustomerPaymentMethodsListResponse list_all_payment_methods_for_a_customer(customer_id, accepted_country, accepted_currency, minimum_amount, maximum_amount, recurring_payment_enabled, installment_payment_enabled)

List payment methods for a Customer

Lists all the applicable payment methods for a particular Customer ID.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.currency import Currency
from hyperswitch.models.customer_payment_methods_list_response import CustomerPaymentMethodsListResponse
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
    api_instance = hyperswitch.PaymentMethodsApi(api_client)
    customer_id = 'customer_id_example' # str | The unique identifier for the customer account
    accepted_country = ['accepted_country_example'] # List[str] | The two-letter ISO currency code
    accepted_currency = [hyperswitch.Currency()] # List[Currency] | The three-letter ISO currency code
    minimum_amount = 56 # int | The minimum amount accepted for processing by the particular payment method.
    maximum_amount = 56 # int | The maximum amount accepted for processing by the particular payment method.
    recurring_payment_enabled = True # bool | Indicates whether the payment method is eligible for recurring payments
    installment_payment_enabled = True # bool | Indicates whether the payment method is eligible for installment payments

    try:
        # List payment methods for a Customer
        api_response = api_instance.list_all_payment_methods_for_a_customer(customer_id, accepted_country, accepted_currency, minimum_amount, maximum_amount, recurring_payment_enabled, installment_payment_enabled)
        print("The response of PaymentMethodsApi->list_all_payment_methods_for_a_customer:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentMethodsApi->list_all_payment_methods_for_a_customer: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **customer_id** | **str**| The unique identifier for the customer account | 
 **accepted_country** | [**List[str]**](str.md)| The two-letter ISO currency code | 
 **accepted_currency** | [**List[Currency]**](Currency.md)| The three-letter ISO currency code | 
 **minimum_amount** | **int**| The minimum amount accepted for processing by the particular payment method. | 
 **maximum_amount** | **int**| The maximum amount accepted for processing by the particular payment method. | 
 **recurring_payment_enabled** | **bool**| Indicates whether the payment method is eligible for recurring payments | 
 **installment_payment_enabled** | **bool**| Indicates whether the payment method is eligible for installment payments | 

### Return type

[**CustomerPaymentMethodsListResponse**](CustomerPaymentMethodsListResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment Methods retrieved |  -  |
**400** | Invalid Data |  -  |
**404** | Payment Methods does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_payment_methods_for_a_customer_0**
> CustomerPaymentMethodsListResponse list_all_payment_methods_for_a_customer_0(client_secret, customer_id, accepted_country, accepted_currency, minimum_amount, maximum_amount, recurring_payment_enabled, installment_payment_enabled)

List customer saved payment methods for a Payment

Lists all the applicable payment methods for a particular payment tied to the `client_secret`.

### Example

* Api Key Authentication (publishable_key):

```python
import hyperswitch
from hyperswitch.models.currency import Currency
from hyperswitch.models.customer_payment_methods_list_response import CustomerPaymentMethodsListResponse
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

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentMethodsApi(api_client)
    client_secret = 'client_secret_example' # str | A secret known only to your client and the authorization server. Used for client side authentication
    customer_id = 'customer_id_example' # str | The unique identifier for the customer account
    accepted_country = ['accepted_country_example'] # List[str] | The two-letter ISO currency code
    accepted_currency = [hyperswitch.Currency()] # List[Currency] | The three-letter ISO currency code
    minimum_amount = 56 # int | The minimum amount accepted for processing by the particular payment method.
    maximum_amount = 56 # int | The maximum amount accepted for processing by the particular payment method.
    recurring_payment_enabled = True # bool | Indicates whether the payment method is eligible for recurring payments
    installment_payment_enabled = True # bool | Indicates whether the payment method is eligible for installment payments

    try:
        # List customer saved payment methods for a Payment
        api_response = api_instance.list_all_payment_methods_for_a_customer_0(client_secret, customer_id, accepted_country, accepted_currency, minimum_amount, maximum_amount, recurring_payment_enabled, installment_payment_enabled)
        print("The response of PaymentMethodsApi->list_all_payment_methods_for_a_customer_0:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentMethodsApi->list_all_payment_methods_for_a_customer_0: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **client_secret** | **str**| A secret known only to your client and the authorization server. Used for client side authentication | 
 **customer_id** | **str**| The unique identifier for the customer account | 
 **accepted_country** | [**List[str]**](str.md)| The two-letter ISO currency code | 
 **accepted_currency** | [**List[Currency]**](Currency.md)| The three-letter ISO currency code | 
 **minimum_amount** | **int**| The minimum amount accepted for processing by the particular payment method. | 
 **maximum_amount** | **int**| The maximum amount accepted for processing by the particular payment method. | 
 **recurring_payment_enabled** | **bool**| Indicates whether the payment method is eligible for recurring payments | 
 **installment_payment_enabled** | **bool**| Indicates whether the payment method is eligible for installment payments | 

### Return type

[**CustomerPaymentMethodsListResponse**](CustomerPaymentMethodsListResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment Methods retrieved for customer tied to its respective client-secret passed in the param |  -  |
**400** | Invalid Data |  -  |
**404** | Payment Methods does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **list_all_payment_methods_for_a_merchant**
> PaymentMethodListResponse list_all_payment_methods_for_a_merchant(account_id, accepted_country, accepted_currency, minimum_amount, maximum_amount, recurring_payment_enabled, installment_payment_enabled)

List payment methods for a Merchant

Lists the applicable payment methods for a particular Merchant ID.
Use the client secret and publishable key authorization to list all relevant payment methods of the merchant for the payment corresponding to the client secret.

### Example

* Api Key Authentication (publishable_key):
* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.currency import Currency
from hyperswitch.models.payment_method_list_response import PaymentMethodListResponse
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

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentMethodsApi(api_client)
    account_id = 'account_id_example' # str | The unique identifier for the merchant account
    accepted_country = ['accepted_country_example'] # List[str] | The two-letter ISO currency code
    accepted_currency = [hyperswitch.Currency()] # List[Currency] | The three-letter ISO currency code
    minimum_amount = 56 # int | The minimum amount accepted for processing by the particular payment method.
    maximum_amount = 56 # int | The maximum amount accepted for processing by the particular payment method.
    recurring_payment_enabled = True # bool | Indicates whether the payment method is eligible for recurring payments
    installment_payment_enabled = True # bool | Indicates whether the payment method is eligible for installment payments

    try:
        # List payment methods for a Merchant
        api_response = api_instance.list_all_payment_methods_for_a_merchant(account_id, accepted_country, accepted_currency, minimum_amount, maximum_amount, recurring_payment_enabled, installment_payment_enabled)
        print("The response of PaymentMethodsApi->list_all_payment_methods_for_a_merchant:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentMethodsApi->list_all_payment_methods_for_a_merchant: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **account_id** | **str**| The unique identifier for the merchant account | 
 **accepted_country** | [**List[str]**](str.md)| The two-letter ISO currency code | 
 **accepted_currency** | [**List[Currency]**](Currency.md)| The three-letter ISO currency code | 
 **minimum_amount** | **int**| The minimum amount accepted for processing by the particular payment method. | 
 **maximum_amount** | **int**| The maximum amount accepted for processing by the particular payment method. | 
 **recurring_payment_enabled** | **bool**| Indicates whether the payment method is eligible for recurring payments | 
 **installment_payment_enabled** | **bool**| Indicates whether the payment method is eligible for installment payments | 

### Return type

[**PaymentMethodListResponse**](PaymentMethodListResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment Methods retrieved |  -  |
**400** | Invalid Data |  -  |
**404** | Payment Methods does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **retrieve_a_payment_method**
> PaymentMethodResponse retrieve_a_payment_method(method_id)

Payment Method - Retrieve

Retrieves a payment method of a customer.

### Example

* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payment_method_response import PaymentMethodResponse
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
    api_instance = hyperswitch.PaymentMethodsApi(api_client)
    method_id = 'method_id_example' # str | The unique identifier for the Payment Method

    try:
        # Payment Method - Retrieve
        api_response = api_instance.retrieve_a_payment_method(method_id)
        print("The response of PaymentMethodsApi->retrieve_a_payment_method:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentMethodsApi->retrieve_a_payment_method: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **method_id** | **str**| The unique identifier for the Payment Method | 

### Return type

[**PaymentMethodResponse**](PaymentMethodResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment Method retrieved |  -  |
**404** | Payment Method does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **set_the_payment_method_as_default**
> CustomerDefaultPaymentMethodResponse set_the_payment_method_as_default(customer_id, payment_method_id)

Payment Method - Set Default Payment Method for Customer

Set the Payment Method as Default for the Customer.

### Example

* Api Key Authentication (ephemeral_key):

```python
import hyperswitch
from hyperswitch.models.customer_default_payment_method_response import CustomerDefaultPaymentMethodResponse
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

# Configure API key authorization: ephemeral_key
configuration.api_key['ephemeral_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['ephemeral_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentMethodsApi(api_client)
    customer_id = 'customer_id_example' # str | The unique identifier for the Customer
    payment_method_id = 'payment_method_id_example' # str | The unique identifier for the Payment Method

    try:
        # Payment Method - Set Default Payment Method for Customer
        api_response = api_instance.set_the_payment_method_as_default(customer_id, payment_method_id)
        print("The response of PaymentMethodsApi->set_the_payment_method_as_default:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentMethodsApi->set_the_payment_method_as_default: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **customer_id** | **str**| The unique identifier for the Customer | 
 **payment_method_id** | **str**| The unique identifier for the Payment Method | 

### Return type

[**CustomerDefaultPaymentMethodResponse**](CustomerDefaultPaymentMethodResponse.md)

### Authorization

[ephemeral_key](../README.md#ephemeral_key)

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment Method has been set as default |  -  |
**400** | Payment Method has already been set as default for that customer |  -  |
**404** | Payment Method not found for the customer |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **update_a_payment_method**
> PaymentMethodResponse update_a_payment_method(method_id, payment_method_update)

Payment Method - Update

Update an existing payment method of a customer.
This API is useful for use cases such as updating the card number for expired cards to prevent discontinuity in recurring payments.

### Example

* Api Key Authentication (publishable_key):
* Api Key Authentication (api_key):

```python
import hyperswitch
from hyperswitch.models.payment_method_response import PaymentMethodResponse
from hyperswitch.models.payment_method_update import PaymentMethodUpdate
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

# Configure API key authorization: publishable_key
configuration.api_key['publishable_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['publishable_key'] = 'Bearer'

# Configure API key authorization: api_key
configuration.api_key['api_key'] = os.environ["API_KEY"]

# Uncomment below to setup prefix (e.g. Bearer) for API key, if needed
# configuration.api_key_prefix['api_key'] = 'Bearer'

# Enter a context with an instance of the API client
with hyperswitch.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = hyperswitch.PaymentMethodsApi(api_client)
    method_id = 'method_id_example' # str | The unique identifier for the Payment Method
    payment_method_update = hyperswitch.PaymentMethodUpdate() # PaymentMethodUpdate | 

    try:
        # Payment Method - Update
        api_response = api_instance.update_a_payment_method(method_id, payment_method_update)
        print("The response of PaymentMethodsApi->update_a_payment_method:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling PaymentMethodsApi->update_a_payment_method: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **method_id** | **str**| The unique identifier for the Payment Method | 
 **payment_method_update** | [**PaymentMethodUpdate**](PaymentMethodUpdate.md)|  | 

### Return type

[**PaymentMethodResponse**](PaymentMethodResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

 - **Content-Type**: application/json
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Payment Method updated |  -  |
**404** | Payment Method does not exist in records |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

