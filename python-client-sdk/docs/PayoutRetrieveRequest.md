# PayoutRetrieveRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payout_id** | **str** | Unique identifier for the payout. This ensures idempotency for multiple payouts that have been done by a single merchant. This field is auto generated and is returned in the API response. | 
**force_sync** | **bool** | &#x60;force_sync&#x60; with the connector to get payout details (defaults to false) | [optional] [default to False]
**merchant_id** | **str** | The identifier for the Merchant Account. | [optional] 

## Example

```python
from hyperswitch.models.payout_retrieve_request import PayoutRetrieveRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutRetrieveRequest from a JSON string
payout_retrieve_request_instance = PayoutRetrieveRequest.from_json(json)
# print the JSON string representation of the object
print(PayoutRetrieveRequest.to_json())

# convert the object into a dict
payout_retrieve_request_dict = payout_retrieve_request_instance.to_dict()
# create an instance of PayoutRetrieveRequest from a dict
payout_retrieve_request_from_dict = PayoutRetrieveRequest.from_dict(payout_retrieve_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


