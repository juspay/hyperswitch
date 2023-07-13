# PaymentMethodCreate

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method** | [**crate::models::PaymentMethodType**](PaymentMethodType.md) |  | 
**payment_method_type** | Option<[**crate::models::PaymentMethodType**](PaymentMethodType.md)> |  | [optional]
**payment_method_issuer** | Option<**String**> | The name of the bank/ provider issuing the payment method to the end user | [optional]
**payment_method_issuer_code** | Option<[**crate::models::PaymentMethodIssuerCode**](PaymentMethodIssuerCode.md)> |  | [optional]
**card** | Option<[**crate::models::CardDetail**](CardDetail.md)> |  | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional]
**customer_id** | Option<**String**> | The unique identifier of the customer. | [optional]
**card_network** | Option<**String**> | The card network | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


