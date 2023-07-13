# \PaymentsApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**cancel_a_payment**](PaymentsApi.md#cancel_a_payment) | **POST** /payments/{payment_id}/cancel | Payments - Cancel
[**capture_a_payment**](PaymentsApi.md#capture_a_payment) | **POST** /payments/{payment_id}/capture | Payments - Capture
[**confirm_a_payment**](PaymentsApi.md#confirm_a_payment) | **POST** /payments/{payment_id}/confirm | Payments - Confirm
[**create_a_payment**](PaymentsApi.md#create_a_payment) | **POST** /payments | Payments - Create
[**create_session_tokens_for_a_payment**](PaymentsApi.md#create_session_tokens_for_a_payment) | **POST** /payments/session_tokens | Payments - Session token
[**list_all_payments**](PaymentsApi.md#list_all_payments) | **GET** /payments/list | Payments - List
[**retrieve_a_payment**](PaymentsApi.md#retrieve_a_payment) | **GET** /payments/{payment_id} | Payments - Retrieve
[**update_a_payment**](PaymentsApi.md#update_a_payment) | **POST** /payments/{payment_id} | Payments - Update



## cancel_a_payment

> cancel_a_payment(payment_id, payments_cancel_request)
Payments - Cancel

Payments - Cancel  A Payment could can be cancelled when it is in one of these statuses: requires_payment_method, requires_capture, requires_confirmation, requires_customer_action

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**payment_id** | **String** | The identifier for payment | [required] |
**payments_cancel_request** | [**PaymentsCancelRequest**](PaymentsCancelRequest.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## capture_a_payment

> crate::models::PaymentsResponse capture_a_payment(payment_id, payments_capture_request)
Payments - Capture

Payments - Capture  To capture the funds for an uncaptured payment

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**payment_id** | **String** | The identifier for payment | [required] |
**payments_capture_request** | [**PaymentsCaptureRequest**](PaymentsCaptureRequest.md) |  | [required] |

### Return type

[**crate::models::PaymentsResponse**](PaymentsResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## confirm_a_payment

> crate::models::PaymentsResponse confirm_a_payment(payment_id, payments_request)
Payments - Confirm

Payments - Confirm  This API is to confirm the payment request and forward payment to the payment processor. This API provides more granular control upon when the API is forwarded to the payment processor. Alternatively you can confirm the payment within the Payments Create API

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**payment_id** | **String** | The identifier for payment | [required] |
**payments_request** | [**PaymentsRequest**](PaymentsRequest.md) |  | [required] |

### Return type

[**crate::models::PaymentsResponse**](PaymentsResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## create_a_payment

> crate::models::PaymentsResponse create_a_payment(payments_create_request)
Payments - Create

Payments - Create  To process a payment you will have to create a payment, attach a payment method and confirm. Depending on the user journey you wish to achieve, you may opt to all the steps in a single request or in a sequence of API request using following APIs: (i) Payments - Update, (ii) Payments - Confirm, and (iii) Payments - Capture

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**payments_create_request** | [**PaymentsCreateRequest**](PaymentsCreateRequest.md) |  | [required] |

### Return type

[**crate::models::PaymentsResponse**](PaymentsResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## create_session_tokens_for_a_payment

> crate::models::PaymentsSessionResponse create_session_tokens_for_a_payment(payments_session_request)
Payments - Session token

Payments - Session token  To create the session object or to get session token for wallets

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**payments_session_request** | [**PaymentsSessionRequest**](PaymentsSessionRequest.md) |  | [required] |

### Return type

[**crate::models::PaymentsSessionResponse**](PaymentsSessionResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_all_payments

> list_all_payments(customer_id, starting_after, ending_before, limit, created, created_lt, created_gt, created_lte, created_gte)
Payments - List

Payments - List  To list the payments

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**customer_id** | **String** | The identifier for the customer | [required] |
**starting_after** | **String** | A cursor for use in pagination, fetch the next list after some object | [required] |
**ending_before** | **String** | A cursor for use in pagination, fetch the previous list before some object | [required] |
**limit** | **i64** | Limit on the number of objects to return | [required] |
**created** | **String** | The time at which payment is created | [required] |
**created_lt** | **String** | Time less than the payment created time | [required] |
**created_gt** | **String** | Time greater than the payment created time | [required] |
**created_lte** | **String** | Time less than or equals to the payment created time | [required] |
**created_gte** | **String** | Time greater than or equals to the payment created time | [required] |

### Return type

 (empty response body)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## retrieve_a_payment

> crate::models::PaymentsResponse retrieve_a_payment(payment_id, payment_retrieve_body)
Payments - Retrieve

Payments - Retrieve  To retrieve the properties of a Payment. This may be used to get the status of a previously initiated payment or next action for an ongoing payment

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**payment_id** | **String** | The identifier for payment | [required] |
**payment_retrieve_body** | [**PaymentRetrieveBody**](PaymentRetrieveBody.md) |  | [required] |

### Return type

[**crate::models::PaymentsResponse**](PaymentsResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_a_payment

> crate::models::PaymentsResponse update_a_payment(payment_id, payments_request)
Payments - Update

Payments - Update  To update the properties of a PaymentIntent object. This may include attaching a payment method, or attaching customer object or metadata fields after the Payment is created

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**payment_id** | **String** | The identifier for payment | [required] |
**payments_request** | [**PaymentsRequest**](PaymentsRequest.md) |  | [required] |

### Return type

[**crate::models::PaymentsResponse**](PaymentsResponse.md)

### Authorization

[publishable_key](../README.md#publishable_key), [api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

