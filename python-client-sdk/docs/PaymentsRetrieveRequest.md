# PaymentsRetrieveRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**resource_id** | **str** | The type of ID (ex: payment intent id, payment attempt id or connector txn id) | 
**merchant_id** | **str** | The identifier for the Merchant Account. | [optional] 
**force_sync** | **bool** | Decider to enable or disable the connector call for retrieve request | 
**param** | **str** | The parameters passed to a retrieve request | [optional] 
**connector** | **str** | The name of the connector | [optional] 
**merchant_connector_details** | [**MerchantConnectorDetailsWrap**](MerchantConnectorDetailsWrap.md) |  | [optional] 
**client_secret** | **str** | This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK | [optional] 
**expand_captures** | **bool** | If enabled provides list of captures linked to latest attempt | [optional] 
**expand_attempts** | **bool** | If enabled provides list of attempts linked to payment intent | [optional] 

## Example

```python
from hyperswitch.models.payments_retrieve_request import PaymentsRetrieveRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsRetrieveRequest from a JSON string
payments_retrieve_request_instance = PaymentsRetrieveRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsRetrieveRequest.to_json())

# convert the object into a dict
payments_retrieve_request_dict = payments_retrieve_request_instance.to_dict()
# create an instance of PaymentsRetrieveRequest from a dict
payments_retrieve_request_from_dict = PaymentsRetrieveRequest.from_dict(payments_retrieve_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


