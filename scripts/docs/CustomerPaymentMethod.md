# CustomerPaymentMethod

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_token** | **String** | Token for payment method in temporary card locker which gets refreshed often | 
**customer_id** | **String** | The unique identifier of the customer. | 
**payment_method** | [**crate::models::PaymentMethodType**](PaymentMethodType.md) |  | 
**payment_method_type** | Option<[**crate::models::PaymentMethodType**](PaymentMethodType.md)> |  | [optional]
**payment_method_issuer** | Option<**String**> | The name of the bank/ provider issuing the payment method to the end user | [optional]
**payment_method_issuer_code** | Option<[**crate::models::PaymentMethodIssuerCode**](PaymentMethodIssuerCode.md)> |  | [optional]
**recurring_enabled** | **bool** | Indicates whether the payment method is eligible for recurring payments | 
**installment_payment_enabled** | **bool** | Indicates whether the payment method is eligible for installment payments | 
**payment_experience** | Option<[**Vec<crate::models::PaymentExperience>**](PaymentExperience.md)> | Type of payment experience enabled with the connector | [optional]
**card** | Option<[**crate::models::CardDetailFromLocker**](CardDetailFromLocker.md)> |  | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional]
**created** | Option<**String**> | A timestamp (ISO 8601 code) that determines when the customer was created | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


