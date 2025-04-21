# BankTransferInstructionsOneOf4


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**multibanco** | [**MultibancoTransferInstructions**](MultibancoTransferInstructions.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_instructions_one_of4 import BankTransferInstructionsOneOf4

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferInstructionsOneOf4 from a JSON string
bank_transfer_instructions_one_of4_instance = BankTransferInstructionsOneOf4.from_json(json)
# print the JSON string representation of the object
print(BankTransferInstructionsOneOf4.to_json())

# convert the object into a dict
bank_transfer_instructions_one_of4_dict = bank_transfer_instructions_one_of4_instance.to_dict()
# create an instance of BankTransferInstructionsOneOf4 from a dict
bank_transfer_instructions_one_of4_from_dict = BankTransferInstructionsOneOf4.from_dict(bank_transfer_instructions_one_of4_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


