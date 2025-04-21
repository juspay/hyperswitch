# BankTransferInstructionsOneOf2


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**sepa_bank_instructions** | [**SepaBankTransferInstructions**](SepaBankTransferInstructions.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_instructions_one_of2 import BankTransferInstructionsOneOf2

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferInstructionsOneOf2 from a JSON string
bank_transfer_instructions_one_of2_instance = BankTransferInstructionsOneOf2.from_json(json)
# print the JSON string representation of the object
print(BankTransferInstructionsOneOf2.to_json())

# convert the object into a dict
bank_transfer_instructions_one_of2_dict = bank_transfer_instructions_one_of2_instance.to_dict()
# create an instance of BankTransferInstructionsOneOf2 from a dict
bank_transfer_instructions_one_of2_from_dict = BankTransferInstructionsOneOf2.from_dict(bank_transfer_instructions_one_of2_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


