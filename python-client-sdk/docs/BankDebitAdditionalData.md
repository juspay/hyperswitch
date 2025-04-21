# BankDebitAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ach** | [**AchBankDebitAdditionalData**](AchBankDebitAdditionalData.md) |  | 
**bacs** | [**BacsBankDebitAdditionalData**](BacsBankDebitAdditionalData.md) |  | 
**becs** | [**BecsBankDebitAdditionalData**](BecsBankDebitAdditionalData.md) |  | 
**sepa** | [**SepaBankDebitAdditionalData**](SepaBankDebitAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.bank_debit_additional_data import BankDebitAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitAdditionalData from a JSON string
bank_debit_additional_data_instance = BankDebitAdditionalData.from_json(json)
# print the JSON string representation of the object
print(BankDebitAdditionalData.to_json())

# convert the object into a dict
bank_debit_additional_data_dict = bank_debit_additional_data_instance.to_dict()
# create an instance of BankDebitAdditionalData from a dict
bank_debit_additional_data_from_dict = BankDebitAdditionalData.from_dict(bank_debit_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


