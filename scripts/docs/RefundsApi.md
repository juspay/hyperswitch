# \RefundsApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**create_a_refund**](RefundsApi.md#create_a_refund) | **POST** /refunds | Refunds - Create
[**list_all_refunds**](RefundsApi.md#list_all_refunds) | **POST** /refunds/list | Refunds - List
[**retrieve_a_refund**](RefundsApi.md#retrieve_a_refund) | **GET** /refunds/{refund_id} | Refunds - Retrieve (GET)
[**update_a_refund**](RefundsApi.md#update_a_refund) | **POST** /refunds/{refund_id} | Refunds - Update



## create_a_refund

> crate::models::RefundResponse create_a_refund(refund_request)
Refunds - Create

Refunds - Create  To create a refund against an already processed payment

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**refund_request** | [**RefundRequest**](RefundRequest.md) |  | [required] |

### Return type

[**crate::models::RefundResponse**](RefundResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## list_all_refunds

> crate::models::RefundListResponse list_all_refunds(refund_list_request)
Refunds - List

Refunds - List  To list the refunds associated with a payment_id or with the merchant, if payment_id is not provided

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**refund_list_request** | [**RefundListRequest**](RefundListRequest.md) |  | [required] |

### Return type

[**crate::models::RefundListResponse**](RefundListResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## retrieve_a_refund

> crate::models::RefundResponse retrieve_a_refund(refund_id)
Refunds - Retrieve (GET)

Refunds - Retrieve (GET)  To retrieve the properties of a Refund. This may be used to get the status of a previously initiated payment or next action for an ongoing payment

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**refund_id** | **String** | The identifier for refund | [required] |

### Return type

[**crate::models::RefundResponse**](RefundResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## update_a_refund

> crate::models::RefundResponse update_a_refund(refund_id, refund_update_request)
Refunds - Update

Refunds - Update  To update the properties of a Refund object. This may include attaching a reason for the refund or metadata fields

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**refund_id** | **String** | The identifier for refund | [required] |
**refund_update_request** | [**RefundUpdateRequest**](RefundUpdateRequest.md) |  | [required] |

### Return type

[**crate::models::RefundResponse**](RefundResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

