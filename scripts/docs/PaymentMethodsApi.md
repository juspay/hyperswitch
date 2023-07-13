# \PaymentMethodsApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_a_payment_method**](PaymentMethodsApi.md#create_a_payment_method) | **POST** /payment_methods | PaymentMethods - Create
[**delete_a_payment_method**](PaymentMethodsApi.md#delete_a_payment_method) | **DELETE** /payment_methods/{method_id} | Payment Method - Delete
[**list_all_payment_methods_for_a_customer**](PaymentMethodsApi.md#list_all_payment_methods_for_a_customer) | **GET** /customer/{customer_id}/payment_methods | List payment methods for a Customer
[**list_all_payment_methods_for_a_merchant**](PaymentMethodsApi.md#list_all_payment_methods_for_a_merchant) | **GET** /account/payment_methods | List payment methods for a Merchant
[**retrieve_a_payment_method**](PaymentMethodsApi.md#retrieve_a_payment_method) | **GET** /payment_methods/{method_id} | Payment Method - Retrieve
[**update_a_payment_method**](PaymentMethodsApi.md#update_a_payment_method) | **POST** /payment_methods/{method_id} | Payment Method - Update



## create_a_payment_method

> crate::models::PaymentMethodResponse create_a_payment_method(payment_method_create)
PaymentMethods - Create

PaymentMethods - Create  To create a payment method against a customer object. In case of cards, this API could be used only by PCI compliant merchants

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**payment_method_create** | [**PaymentMethodCreate**](PaymentMethodCreate.md) |  | [required] |

### Return type

[**crate::models::PaymentMethodResponse**](PaymentMethodResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## delete_a_payment_method

> crate::models::PaymentMethodDeleteResponse delete_a_payment_method(method_id)
Payment Method - Delete

Payment Method - Delete  Delete payment method

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**method_id** | **String** | The unique identifier for the Payment Method | [required] |

### Return type

[**crate::models::PaymentMethodDeleteResponse**](PaymentMethodDeleteResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_all_payment_methods_for_a_customer

> crate::models::CustomerPaymentMethodsListResponse list_all_payment_methods_for_a_customer(customer_id, accepted_country, accepted_currency, minimum_amount, maximum_amount, recurring_payment_enabled, installment_payment_enabled)
List payment methods for a Customer

List payment methods for a Customer  To filter and list the applicable payment methods for a particular Customer ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**customer_id** | **String** | The unique identifier for the customer account | [required] |
**accepted_country** | [**Vec<String>**](String.md) | The two-letter ISO currency code | [required] |
**accepted_currency** | [**Vec<crate::models::Currency>**](crate::models::Currency.md) | The three-letter ISO currency code | [required] |
**minimum_amount** | **i64** | The minimum amount accepted for processing by the particular payment method. | [required] |
**maximum_amount** | **i64** | The maximum amount amount accepted for processing by the particular payment method. | [required] |
**recurring_payment_enabled** | **bool** | Indicates whether the payment method is eligible for recurring payments | [required] |
**installment_payment_enabled** | **bool** | Indicates whether the payment method is eligible for installment payments | [required] |

### Return type

[**crate::models::CustomerPaymentMethodsListResponse**](CustomerPaymentMethodsListResponse.md)

### Authorization

[api_key](../README.md#api_key), [ephemeral_key](../README.md#ephemeral_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_all_payment_methods_for_a_merchant

> crate::models::PaymentMethodListResponse list_all_payment_methods_for_a_merchant(account_id, accepted_country, accepted_currency, minimum_amount, maximum_amount, recurring_payment_enabled, installment_payment_enabled)
List payment methods for a Merchant

List payment methods for a Merchant  To filter and list the applicable payment methods for a particular Merchant ID

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**account_id** | **String** | The unique identifier for the merchant account | [required] |
**accepted_country** | [**Vec<String>**](String.md) | The two-letter ISO currency code | [required] |
**accepted_currency** | [**Vec<crate::models::Currency>**](crate::models::Currency.md) | The three-letter ISO currency code | [required] |
**minimum_amount** | **i64** | The minimum amount accepted for processing by the particular payment method. | [required] |
**maximum_amount** | **i64** | The maximum amount amount accepted for processing by the particular payment method. | [required] |
**recurring_payment_enabled** | **bool** | Indicates whether the payment method is eligible for recurring payments | [required] |
**installment_payment_enabled** | **bool** | Indicates whether the payment method is eligible for installment payments | [required] |

### Return type

[**crate::models::PaymentMethodListResponse**](PaymentMethodListResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## retrieve_a_payment_method

> crate::models::PaymentMethodResponse retrieve_a_payment_method(method_id)
Payment Method - Retrieve

Payment Method - Retrieve  To retrieve a payment method

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**method_id** | **String** | The unique identifier for the Payment Method | [required] |

### Return type

[**crate::models::PaymentMethodResponse**](PaymentMethodResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_a_payment_method

> crate::models::PaymentMethodResponse update_a_payment_method(method_id, payment_method_update)
Payment Method - Update

Payment Method - Update  To update an existing payment method attached to a customer object. This API is useful for use cases such as updating the card number for expired cards to prevent discontinuity in recurring payments

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**method_id** | **String** | The unique identifier for the Payment Method | [required] |
**payment_method_update** | [**PaymentMethodUpdate**](PaymentMethodUpdate.md) |  | [required] |

### Return type

[**crate::models::PaymentMethodResponse**](PaymentMethodResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

