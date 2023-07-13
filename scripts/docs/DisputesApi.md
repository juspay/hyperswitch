# \DisputesApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**list_disputes**](DisputesApi.md#list_disputes) | **GET** /disputes/list | Disputes - List Disputes
[**retrieve_a_dispute**](DisputesApi.md#retrieve_a_dispute) | **GET** /disputes/{dispute_id} | Disputes - Retrieve Dispute



## list_disputes

> Vec<crate::models::DisputeResponse> list_disputes(limit, dispute_status, dispute_stage, reason, connector, received_time, received_time_period_lt, received_time_period_gt, received_time_period_lte, received_time_period_gte)
Disputes - List Disputes

Disputes - List Disputes

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**limit** | Option<**i64**> | The maximum number of Dispute Objects to include in the response |  |
**dispute_status** | Option<[**crate::models::DisputeStatus**](.md)> | The status of dispute |  |
**dispute_stage** | Option<[**crate::models::DisputeStage**](.md)> | The stage of dispute |  |
**reason** | Option<**String**> | The reason for dispute |  |
**connector** | Option<**String**> | The connector linked to dispute |  |
**received_time** | Option<**String**> | The time at which dispute is received |  |
**received_time_period_lt** | Option<**String**> | Time less than the dispute received time |  |
**received_time_period_gt** | Option<**String**> | Time greater than the dispute received time |  |
**received_time_period_lte** | Option<**String**> | Time less than or equals to the dispute received time |  |
**received_time_period_gte** | Option<**String**> | Time greater than or equals to the dispute received time |  |

### Return type

[**Vec<crate::models::DisputeResponse>**](DisputeResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## retrieve_a_dispute

> crate::models::DisputeResponse retrieve_a_dispute(dispute_id)
Disputes - Retrieve Dispute

Disputes - Retrieve Dispute

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**dispute_id** | **String** | The identifier for dispute | [required] |

### Return type

[**crate::models::DisputeResponse**](DisputeResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

