# BankTransferDataOneOf1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**sepa_bank_transfer** | [**BankTransferDataOneOf1SepaBankTransfer**](BankTransferDataOneOf1SepaBankTransfer.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_data_one_of1 import BankTransferDataOneOf1

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferDataOneOf1 from a JSON string
bank_transfer_data_one_of1_instance = BankTransferDataOneOf1.from_json(json)
# print the JSON string representation of the object
print(BankTransferDataOneOf1.to_json())

# convert the object into a dict
bank_transfer_data_one_of1_dict = bank_transfer_data_one_of1_instance.to_dict()
# create an instance of BankTransferDataOneOf1 from a dict
bank_transfer_data_one_of1_from_dict = BankTransferDataOneOf1.from_dict(bank_transfer_data_one_of1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


