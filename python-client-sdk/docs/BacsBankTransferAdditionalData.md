# BacsBankTransferAdditionalData

Masked payout method details for bacs bank transfer payout method

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_sort_code** | **str** | Partially masked sort code for Bacs payment method | 
**bank_account_number** | **str** | Bank account&#39;s owner name | 
**bank_name** | **str** | Bank name | [optional] 
**bank_country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**bank_city** | **str** | Bank city | [optional] 

## Example

```python
from hyperswitch.models.bacs_bank_transfer_additional_data import BacsBankTransferAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of BacsBankTransferAdditionalData from a JSON string
bacs_bank_transfer_additional_data_instance = BacsBankTransferAdditionalData.from_json(json)
# print the JSON string representation of the object
print(BacsBankTransferAdditionalData.to_json())

# convert the object into a dict
bacs_bank_transfer_additional_data_dict = bacs_bank_transfer_additional_data_instance.to_dict()
# create an instance of BacsBankTransferAdditionalData from a dict
bacs_bank_transfer_additional_data_from_dict = BacsBankTransferAdditionalData.from_dict(bacs_bank_transfer_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


