# AchBankTransferAdditionalData

Masked payout method details for ach bank transfer payout method

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_account_number** | **str** | Partially masked account number for ach bank debit payment | 
**bank_routing_number** | **str** | Partially masked routing number for ach bank debit payment | 
**bank_name** | [**BankNames**](BankNames.md) |  | [optional] 
**bank_country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**bank_city** | **str** | Bank city | [optional] 

## Example

```python
from hyperswitch.models.ach_bank_transfer_additional_data import AchBankTransferAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of AchBankTransferAdditionalData from a JSON string
ach_bank_transfer_additional_data_instance = AchBankTransferAdditionalData.from_json(json)
# print the JSON string representation of the object
print(AchBankTransferAdditionalData.to_json())

# convert the object into a dict
ach_bank_transfer_additional_data_dict = ach_bank_transfer_additional_data_instance.to_dict()
# create an instance of AchBankTransferAdditionalData from a dict
ach_bank_transfer_additional_data_from_dict = AchBankTransferAdditionalData.from_dict(ach_bank_transfer_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


