# \MandatesApi

All URIs are relative to *https://sandbox.hyperswitch.io*

Method | HTTP request | Description
------------- | ------------- | -------------
[**retrieve_a_mandate**](MandatesApi.md#retrieve_a_mandate) | **GET** /mandates/{mandate_id} | Mandates - Retrieve Mandate
[**revoke_a_mandate**](MandatesApi.md#revoke_a_mandate) | **POST** /mandates/revoke/{mandate_id} | Mandates - Revoke Mandate



## retrieve_a_mandate

> crate::models::MandateResponse retrieve_a_mandate(mandate_id)
Mandates - Retrieve Mandate

Mandates - Retrieve Mandate  Retrieve a mandate

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**mandate_id** | **String** | The identifier for mandate | [required] |

### Return type

[**crate::models::MandateResponse**](MandateResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## revoke_a_mandate

> crate::models::MandateRevokedResponse revoke_a_mandate(mandate_id)
Mandates - Revoke Mandate

Mandates - Revoke Mandate  Revoke a mandate

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**mandate_id** | **String** | The identifier for mandate | [required] |

### Return type

[**crate::models::MandateRevokedResponse**](MandateRevokedResponse.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

