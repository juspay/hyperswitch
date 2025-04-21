# BankTransferInstructionsOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**doku_bank_transfer_instructions** | [**DokuBankTransferInstructions**](DokuBankTransferInstructions.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_instructions_one_of import BankTransferInstructionsOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferInstructionsOneOf from a JSON string
bank_transfer_instructions_one_of_instance = BankTransferInstructionsOneOf.from_json(json)
# print the JSON string representation of the object
print(BankTransferInstructionsOneOf.to_json())

# convert the object into a dict
bank_transfer_instructions_one_of_dict = bank_transfer_instructions_one_of_instance.to_dict()
# create an instance of BankTransferInstructionsOneOf from a dict
bank_transfer_instructions_one_of_from_dict = BankTransferInstructionsOneOf.from_dict(bank_transfer_instructions_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


