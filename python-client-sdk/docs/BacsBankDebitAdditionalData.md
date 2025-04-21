# BacsBankDebitAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account_number** | **str** | Partially masked account number for Bacs payment method | 
**sort_code** | **str** | Partially masked sort code for Bacs payment method | 
**bank_account_holder_name** | **str** | Bank account&#39;s owner name | [optional] 

## Example

```python
from hyperswitch.models.bacs_bank_debit_additional_data import BacsBankDebitAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of BacsBankDebitAdditionalData from a JSON string
bacs_bank_debit_additional_data_instance = BacsBankDebitAdditionalData.from_json(json)
# print the JSON string representation of the object
print(BacsBankDebitAdditionalData.to_json())

# convert the object into a dict
bacs_bank_debit_additional_data_dict = bacs_bank_debit_additional_data_instance.to_dict()
# create an instance of BacsBankDebitAdditionalData from a dict
bacs_bank_debit_additional_data_from_dict = BacsBankDebitAdditionalData.from_dict(bacs_bank_debit_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


