# BankTransferInstructionsOneOf3


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bacs_bank_instructions** | [**BacsBankTransferInstructions**](BacsBankTransferInstructions.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_instructions_one_of3 import BankTransferInstructionsOneOf3

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferInstructionsOneOf3 from a JSON string
bank_transfer_instructions_one_of3_instance = BankTransferInstructionsOneOf3.from_json(json)
# print the JSON string representation of the object
print(BankTransferInstructionsOneOf3.to_json())

# convert the object into a dict
bank_transfer_instructions_one_of3_dict = bank_transfer_instructions_one_of3_instance.to_dict()
# create an instance of BankTransferInstructionsOneOf3 from a dict
bank_transfer_instructions_one_of3_from_dict = BankTransferInstructionsOneOf3.from_dict(bank_transfer_instructions_one_of3_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


