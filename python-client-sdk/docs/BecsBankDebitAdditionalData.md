# BecsBankDebitAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account_number** | **str** | Partially masked account number for Becs payment method | 
**bsb_number** | **str** | Bank-State-Branch (bsb) number | 
**bank_account_holder_name** | **str** | Bank account&#39;s owner name | [optional] 

## Example

```python
from hyperswitch.models.becs_bank_debit_additional_data import BecsBankDebitAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of BecsBankDebitAdditionalData from a JSON string
becs_bank_debit_additional_data_instance = BecsBankDebitAdditionalData.from_json(json)
# print the JSON string representation of the object
print(BecsBankDebitAdditionalData.to_json())

# convert the object into a dict
becs_bank_debit_additional_data_dict = becs_bank_debit_additional_data_instance.to_dict()
# create an instance of BecsBankDebitAdditionalData from a dict
becs_bank_debit_additional_data_from_dict = BecsBankDebitAdditionalData.from_dict(becs_bank_debit_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


