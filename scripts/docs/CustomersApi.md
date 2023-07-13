# \CustomersApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_a_customer**](CustomersApi.md#create_a_customer) | **POST** /customers | Create Customer
[**delete_a_customer**](CustomersApi.md#delete_a_customer) | **DELETE** /customers/{customer_id} | Delete Customer
[**retrieve_a_customer**](CustomersApi.md#retrieve_a_customer) | **GET** /customers/{customer_id} | Retrieve Customer
[**update_a_customer**](CustomersApi.md#update_a_customer) | **POST** /customers/{customer_id} | Update Customer



## create_a_customer

> crate::models::CustomerResponse create_a_customer(customer_request)
Create Customer

Create Customer  Create a customer object and store the customer details to be reused for future payments. Incase the customer already exists in the system, this API will respond with the customer details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**customer_request** | [**CustomerRequest**](CustomerRequest.md) |  | [required] |

### Return type

[**crate::models::CustomerResponse**](CustomerResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_a_customer

> crate::models::CustomerDeleteResponse delete_a_customer(customer_id)
Delete Customer

Delete Customer  Delete a customer record.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**customer_id** | **String** | The unique identifier for the Customer | [required] |

### Return type

[**crate::models::CustomerDeleteResponse**](CustomerDeleteResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## retrieve_a_customer

> crate::models::CustomerResponse retrieve_a_customer(customer_id)
Retrieve Customer

Retrieve Customer  Retrieve a customer's details.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**customer_id** | **String** | The unique identifier for the Customer | [required] |

### Return type

[**crate::models::CustomerResponse**](CustomerResponse.md)

### Authorization

[api_key](../README.md#api_key), [ephemeral_key](../README.md#ephemeral_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_a_customer

> crate::models::CustomerResponse update_a_customer(customer_id, customer_request)
Update Customer

Update Customer  Updates the customer's details in a customer object.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**customer_id** | **String** | The unique identifier for the Customer | [required] |
**customer_request** | [**CustomerRequest**](CustomerRequest.md) |  | [required] |

### Return type

[**crate::models::CustomerResponse**](CustomerResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

