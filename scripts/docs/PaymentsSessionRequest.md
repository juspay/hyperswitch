# PaymentsSessionRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_id** | **String** | The identifier for the payment | 
**client_secret** | **String** | This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK | 
**wallets** | [**Vec<crate::models::PaymentMethodType>**](PaymentMethodType.md) | The list of the supported wallets | 
**merchant_connector_details** | Option<[**crate::models::MerchantConnectorDetailsWrap**](MerchantConnectorDetailsWrap.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


