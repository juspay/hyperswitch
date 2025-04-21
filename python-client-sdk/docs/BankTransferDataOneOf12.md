# BankTransferDataOneOf12


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**local_bank_transfer** | [**BankTransferDataOneOf12LocalBankTransfer**](BankTransferDataOneOf12LocalBankTransfer.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_data_one_of12 import BankTransferDataOneOf12

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferDataOneOf12 from a JSON string
bank_transfer_data_one_of12_instance = BankTransferDataOneOf12.from_json(json)
# print the JSON string representation of the object
print(BankTransferDataOneOf12.to_json())

# convert the object into a dict
bank_transfer_data_one_of12_dict = bank_transfer_data_one_of12_instance.to_dict()
# create an instance of BankTransferDataOneOf12 from a dict
bank_transfer_data_one_of12_from_dict = BankTransferDataOneOf12.from_dict(bank_transfer_data_one_of12_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


