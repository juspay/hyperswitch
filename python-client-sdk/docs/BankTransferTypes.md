# BankTransferTypes


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**eligible_connectors** | **List[str]** | The list of eligible connectors for a given payment experience | 

## Example

```python
from hyperswitch.models.bank_transfer_types import BankTransferTypes

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferTypes from a JSON string
bank_transfer_types_instance = BankTransferTypes.from_json(json)
# print the JSON string representation of the object
print(BankTransferTypes.to_json())

# convert the object into a dict
bank_transfer_types_dict = bank_transfer_types_instance.to_dict()
# create an instance of BankTransferTypes from a dict
bank_transfer_types_from_dict = BankTransferTypes.from_dict(bank_transfer_types_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


