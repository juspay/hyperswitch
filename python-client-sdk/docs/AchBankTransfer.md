# AchBankTransfer


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_name** | **str** | Bank name | [optional] 
**bank_country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**bank_city** | **str** | Bank city | [optional] 
**bank_account_number** | **str** | Bank account number is an unique identifier assigned by a bank to a customer. | 
**bank_routing_number** | **str** | [9 digits] Routing number - used in USA for identifying a specific bank. | 

## Example

```python
from hyperswitch.models.ach_bank_transfer import AchBankTransfer

# TODO update the JSON string below
json = "{}"
# create an instance of AchBankTransfer from a JSON string
ach_bank_transfer_instance = AchBankTransfer.from_json(json)
# print the JSON string representation of the object
print(AchBankTransfer.to_json())

# convert the object into a dict
ach_bank_transfer_dict = ach_bank_transfer_instance.to_dict()
# create an instance of AchBankTransfer from a dict
ach_bank_transfer_from_dict = AchBankTransfer.from_dict(ach_bank_transfer_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


