# PayoutRetrieveBody


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**force_sync** | **bool** |  | [optional] 
**merchant_id** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.payout_retrieve_body import PayoutRetrieveBody

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutRetrieveBody from a JSON string
payout_retrieve_body_instance = PayoutRetrieveBody.from_json(json)
# print the JSON string representation of the object
print(PayoutRetrieveBody.to_json())

# convert the object into a dict
payout_retrieve_body_dict = payout_retrieve_body_instance.to_dict()
# create an instance of PayoutRetrieveBody from a dict
payout_retrieve_body_from_dict = PayoutRetrieveBody.from_dict(payout_retrieve_body_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


