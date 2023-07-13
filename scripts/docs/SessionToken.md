# SessionToken

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**delayed_session_token** | **bool** | Identifier for the delayed session response | 
**connector** | **String** | The session token is w.r.t this connector | 
**sdk_next_action** | [**crate::models::SdkNextAction**](SdkNextAction.md) |  | 
**merchant_info** | [**crate::models::GpayMerchantInfo**](GpayMerchantInfo.md) |  | 
**allowed_payment_methods** | [**Vec<crate::models::GpayAllowedPaymentMethods>**](GpayAllowedPaymentMethods.md) | List of the allowed payment meythods | 
**transaction_info** | [**crate::models::GpayTransactionInfo**](GpayTransactionInfo.md) |  | 
**secrets** | Option<[**crate::models::SecretInfoToInitiateSdk**](SecretInfoToInitiateSdk.md)> |  | [optional]
**wallet_name** | **String** |  | 
**session_token** | **String** | The session token for PayPal | 
**session_id** | **String** | The identifier for the session | 
**session_token_data** | [**crate::models::ApplePaySessionResponse**](ApplePaySessionResponse.md) |  | 
**payment_request_data** | Option<[**crate::models::ApplePayPaymentRequest**](ApplePayPaymentRequest.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


