# BankDebitTypes


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**eligible_connectors** | **List[str]** |  | 

## Example

```python
from hyperswitch.models.bank_debit_types import BankDebitTypes

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitTypes from a JSON string
bank_debit_types_instance = BankDebitTypes.from_json(json)
# print the JSON string representation of the object
print(BankDebitTypes.to_json())

# convert the object into a dict
bank_debit_types_dict = bank_debit_types_instance.to_dict()
# create an instance of BankDebitTypes from a dict
bank_debit_types_from_dict = BankDebitTypes.from_dict(bank_debit_types_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


