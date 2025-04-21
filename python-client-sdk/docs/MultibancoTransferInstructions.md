# MultibancoTransferInstructions


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**reference** | **str** |  | 
**entity** | **str** |  | 

## Example

```python
from hyperswitch.models.multibanco_transfer_instructions import MultibancoTransferInstructions

# TODO update the JSON string below
json = "{}"
# create an instance of MultibancoTransferInstructions from a JSON string
multibanco_transfer_instructions_instance = MultibancoTransferInstructions.from_json(json)
# print the JSON string representation of the object
print(MultibancoTransferInstructions.to_json())

# convert the object into a dict
multibanco_transfer_instructions_dict = multibanco_transfer_instructions_instance.to_dict()
# create an instance of MultibancoTransferInstructions from a dict
multibanco_transfer_instructions_from_dict = MultibancoTransferInstructions.from_dict(multibanco_transfer_instructions_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


