# PayoutCreateResponse1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**object** | [**PayoutCreateResponse**](PayoutCreateResponse.md) |  | 

## Example

```python
from hyperswitch.models.payout_create_response1 import PayoutCreateResponse1

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutCreateResponse1 from a JSON string
payout_create_response1_instance = PayoutCreateResponse1.from_json(json)
# print the JSON string representation of the object
print(PayoutCreateResponse1.to_json())

# convert the object into a dict
payout_create_response1_dict = payout_create_response1_instance.to_dict()
# create an instance of PayoutCreateResponse1 from a dict
payout_create_response1_from_dict = PayoutCreateResponse1.from_dict(payout_create_response1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


