# PayoutLinkInitiateRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** |  | 
**payout_id** | **str** |  | 

## Example

```python
from hyperswitch.models.payout_link_initiate_request import PayoutLinkInitiateRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutLinkInitiateRequest from a JSON string
payout_link_initiate_request_instance = PayoutLinkInitiateRequest.from_json(json)
# print the JSON string representation of the object
print(PayoutLinkInitiateRequest.to_json())

# convert the object into a dict
payout_link_initiate_request_dict = payout_link_initiate_request_instance.to_dict()
# create an instance of PayoutLinkInitiateRequest from a dict
payout_link_initiate_request_from_dict = PayoutLinkInitiateRequest.from_dict(payout_link_initiate_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


