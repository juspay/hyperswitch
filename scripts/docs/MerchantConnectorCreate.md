# MerchantConnectorCreate

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector_type** | [**crate::models::ConnectorType**](ConnectorType.md) |  | 
**connector_name** | [**crate::models::Connector**](Connector.md) |  | 
**connector_label** | **String** |  | 
**merchant_connector_id** | Option<**String**> | Unique ID of the connector | [optional]
**connector_account_details** | Option<[**serde_json::Value**](.md)> | Account details of the Connector. You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Useful for storing additional, structured information on an object. | [optional]
**test_mode** | Option<**bool**> | A boolean value to indicate if the connector is in Test mode. By default, its value is false. | [optional][default to false]
**disabled** | Option<**bool**> | A boolean value to indicate if the connector is disabled. By default, its value is false. | [optional][default to false]
**payment_methods_enabled** | Option<[**Vec<crate::models::PaymentMethodsEnabled>**](PaymentMethodsEnabled.md)> | Refers to the Parent Merchant ID if the merchant being created is a sub-merchant | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional]
**frm_configs** | Option<[**crate::models::FrmConfigs**](FrmConfigs.md)> |  | [optional]
**business_country** | Option<[**crate::models::CountryAlpha2**](CountryAlpha2.md)> |  | [optional]
**business_label** | Option<**String**> |  | [optional]
**business_sub_label** | Option<**String**> | Business Sub label of the merchant | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


