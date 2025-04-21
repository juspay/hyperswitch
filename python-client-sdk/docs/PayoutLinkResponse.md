# PayoutLinkResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payout_link_id** | **str** |  | 
**link** | **str** |  | 

## Example

```python
from hyperswitch.models.payout_link_response import PayoutLinkResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutLinkResponse from a JSON string
payout_link_response_instance = PayoutLinkResponse.from_json(json)
# print the JSON string representation of the object
print(PayoutLinkResponse.to_json())

# convert the object into a dict
payout_link_response_dict = payout_link_response_instance.to_dict()
# create an instance of PayoutLinkResponse from a dict
payout_link_response_from_dict = PayoutLinkResponse.from_dict(payout_link_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


