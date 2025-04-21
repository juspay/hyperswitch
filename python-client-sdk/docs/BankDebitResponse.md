# BankDebitResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ach** | [**AchBankDebitAdditionalData**](AchBankDebitAdditionalData.md) |  | 
**bacs** | [**BacsBankDebitAdditionalData**](BacsBankDebitAdditionalData.md) |  | 
**becs** | [**BecsBankDebitAdditionalData**](BecsBankDebitAdditionalData.md) |  | 
**sepa** | [**SepaBankDebitAdditionalData**](SepaBankDebitAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.bank_debit_response import BankDebitResponse

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitResponse from a JSON string
bank_debit_response_instance = BankDebitResponse.from_json(json)
# print the JSON string representation of the object
print(BankDebitResponse.to_json())

# convert the object into a dict
bank_debit_response_dict = bank_debit_response_instance.to_dict()
# create an instance of BankDebitResponse from a dict
bank_debit_response_from_dict = BankDebitResponse.from_dict(bank_debit_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


