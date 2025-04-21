# BankTransfer


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_transfer** | [**BankTransferData**](BankTransferData.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer import BankTransfer

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransfer from a JSON string
bank_transfer_instance = BankTransfer.from_json(json)
# print the JSON string representation of the object
print(BankTransfer.to_json())

# convert the object into a dict
bank_transfer_dict = bank_transfer_instance.to_dict()
# create an instance of BankTransfer from a dict
bank_transfer_from_dict = BankTransfer.from_dict(bank_transfer_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


