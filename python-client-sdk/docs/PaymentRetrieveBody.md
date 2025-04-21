# PaymentRetrieveBody


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The identifier for the Merchant Account. | [optional] 
**force_sync** | **bool** | Decider to enable or disable the connector call for retrieve request | [optional] 
**client_secret** | **str** | This is a token which expires after 15 minutes, used from the client to authenticate and create sessions from the SDK | [optional] 
**expand_captures** | **bool** | If enabled provides list of captures linked to latest attempt | [optional] 
**expand_attempts** | **bool** | If enabled provides list of attempts linked to payment intent | [optional] 

## Example

```python
from hyperswitch.models.payment_retrieve_body import PaymentRetrieveBody

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentRetrieveBody from a JSON string
payment_retrieve_body_instance = PaymentRetrieveBody.from_json(json)
# print the JSON string representation of the object
print(PaymentRetrieveBody.to_json())

# convert the object into a dict
payment_retrieve_body_dict = payment_retrieve_body_instance.to_dict()
# create an instance of PaymentRetrieveBody from a dict
payment_retrieve_body_from_dict = PaymentRetrieveBody.from_dict(payment_retrieve_body_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


