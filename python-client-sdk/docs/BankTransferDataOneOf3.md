# BankTransferDataOneOf3


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**multibanco_bank_transfer** | [**BankTransferDataOneOf3MultibancoBankTransfer**](BankTransferDataOneOf3MultibancoBankTransfer.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_data_one_of3 import BankTransferDataOneOf3

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferDataOneOf3 from a JSON string
bank_transfer_data_one_of3_instance = BankTransferDataOneOf3.from_json(json)
# print the JSON string representation of the object
print(BankTransferDataOneOf3.to_json())

# convert the object into a dict
bank_transfer_data_one_of3_dict = bank_transfer_data_one_of3_instance.to_dict()
# create an instance of BankTransferDataOneOf3 from a dict
bank_transfer_data_one_of3_from_dict = BankTransferDataOneOf3.from_dict(bank_transfer_data_one_of3_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


