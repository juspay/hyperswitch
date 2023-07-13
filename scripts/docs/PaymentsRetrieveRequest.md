# PaymentsRetrieveRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**resource_id** | [**crate::models::PaymentIdType**](PaymentIdType.md) |  | 
**merchant_id** | Option<**String**> | The identifier for the Merchant Account. | [optional]
**force_sync** | **bool** | Decider to enable or disable the connector call for retrieve request | 
**param** | Option<**String**> | The parameters passed to a retrieve request | [optional]
**connector** | Option<**String**> | The name of the connector | [optional]
**merchant_connector_details** | Option<[**crate::models::MerchantConnectorDetailsWrap**](MerchantConnectorDetailsWrap.md)> |  | [optional]
**client_secret** | Option<**String**> | This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


