# AchTransfer


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account_number** | **str** |  | 
**bank_name** | **str** |  | 
**routing_number** | **str** |  | 
**swift_code** | **str** |  | 

## Example

```python
from hyperswitch.models.ach_transfer import AchTransfer

# TODO update the JSON string below
json = "{}"
# create an instance of AchTransfer from a JSON string
ach_transfer_instance = AchTransfer.from_json(json)
# print the JSON string representation of the object
print(AchTransfer.to_json())

# convert the object into a dict
ach_transfer_dict = ach_transfer_instance.to_dict()
# create an instance of AchTransfer from a dict
ach_transfer_from_dict = AchTransfer.from_dict(ach_transfer_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


