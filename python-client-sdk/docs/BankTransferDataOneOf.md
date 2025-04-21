# BankTransferDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ach_bank_transfer** | [**BankTransferDataOneOfAchBankTransfer**](BankTransferDataOneOfAchBankTransfer.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_data_one_of import BankTransferDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferDataOneOf from a JSON string
bank_transfer_data_one_of_instance = BankTransferDataOneOf.from_json(json)
# print the JSON string representation of the object
print(BankTransferDataOneOf.to_json())

# convert the object into a dict
bank_transfer_data_one_of_dict = bank_transfer_data_one_of_instance.to_dict()
# create an instance of BankTransferDataOneOf from a dict
bank_transfer_data_one_of_from_dict = BankTransferDataOneOf.from_dict(bank_transfer_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


