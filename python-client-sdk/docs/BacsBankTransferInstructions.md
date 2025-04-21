# BacsBankTransferInstructions


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account_holder_name** | **str** |  | 
**account_number** | **str** |  | 
**sort_code** | **str** |  | 

## Example

```python
from hyperswitch.models.bacs_bank_transfer_instructions import BacsBankTransferInstructions

# TODO update the JSON string below
json = "{}"
# create an instance of BacsBankTransferInstructions from a JSON string
bacs_bank_transfer_instructions_instance = BacsBankTransferInstructions.from_json(json)
# print the JSON string representation of the object
print(BacsBankTransferInstructions.to_json())

# convert the object into a dict
bacs_bank_transfer_instructions_dict = bacs_bank_transfer_instructions_instance.to_dict()
# create an instance of BacsBankTransferInstructions from a dict
bacs_bank_transfer_instructions_from_dict = BacsBankTransferInstructions.from_dict(bacs_bank_transfer_instructions_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


