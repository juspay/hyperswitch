# GooglePaySessionResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_info** | [**crate::models::GpayMerchantInfo**](GpayMerchantInfo.md) |  | 
**allowed_payment_methods** | [**Vec<crate::models::GpayAllowedPaymentMethods>**](GpayAllowedPaymentMethods.md) | List of the allowed payment meythods | 
**transaction_info** | [**crate::models::GpayTransactionInfo**](GpayTransactionInfo.md) |  | 
**delayed_session_token** | **bool** | Identifier for the delayed session response | 
**connector** | **String** | The name of the connector | 
**sdk_next_action** | [**crate::models::SdkNextAction**](SdkNextAction.md) |  | 
**secrets** | Option<[**crate::models::SecretInfoToInitiateSdk**](SecretInfoToInitiateSdk.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


