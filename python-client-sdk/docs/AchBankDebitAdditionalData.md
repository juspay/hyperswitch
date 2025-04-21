# AchBankDebitAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account_number** | **str** | Partially masked account number for ach bank debit payment | 
**routing_number** | **str** | Partially masked routing number for ach bank debit payment | 
**card_holder_name** | **str** | Card holder&#39;s name | [optional] 
**bank_account_holder_name** | **str** | Bank account&#39;s owner name | [optional] 
**bank_name** | [**BankNames**](BankNames.md) |  | [optional] 
**bank_type** | [**BankType**](BankType.md) |  | [optional] 
**bank_holder_type** | [**BankHolderType**](BankHolderType.md) |  | [optional] 

## Example

```python
from hyperswitch.models.ach_bank_debit_additional_data import AchBankDebitAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of AchBankDebitAdditionalData from a JSON string
ach_bank_debit_additional_data_instance = AchBankDebitAdditionalData.from_json(json)
# print the JSON string representation of the object
print(AchBankDebitAdditionalData.to_json())

# convert the object into a dict
ach_bank_debit_additional_data_dict = ach_bank_debit_additional_data_instance.to_dict()
# create an instance of AchBankDebitAdditionalData from a dict
ach_bank_debit_additional_data_from_dict = AchBankDebitAdditionalData.from_dict(ach_bank_debit_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


