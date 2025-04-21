# BankTransferInstructionsOneOf1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ach_credit_transfer** | [**AchTransfer**](AchTransfer.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_instructions_one_of1 import BankTransferInstructionsOneOf1

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferInstructionsOneOf1 from a JSON string
bank_transfer_instructions_one_of1_instance = BankTransferInstructionsOneOf1.from_json(json)
# print the JSON string representation of the object
print(BankTransferInstructionsOneOf1.to_json())

# convert the object into a dict
bank_transfer_instructions_one_of1_dict = bank_transfer_instructions_one_of1_instance.to_dict()
# create an instance of BankTransferInstructionsOneOf1 from a dict
bank_transfer_instructions_one_of1_from_dict = BankTransferInstructionsOneOf1.from_dict(bank_transfer_instructions_one_of1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


