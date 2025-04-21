# BacsBankTransfer


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_name** | **str** | Bank name | [optional] 
**bank_country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**bank_city** | **str** | Bank city | [optional] 
**bank_account_number** | **str** | Bank account number is an unique identifier assigned by a bank to a customer. | 
**bank_sort_code** | **str** | [6 digits] Sort Code - used in UK and Ireland for identifying a bank and it&#39;s branches. | 

## Example

```python
from hyperswitch.models.bacs_bank_transfer import BacsBankTransfer

# TODO update the JSON string below
json = "{}"
# create an instance of BacsBankTransfer from a JSON string
bacs_bank_transfer_instance = BacsBankTransfer.from_json(json)
# print the JSON string representation of the object
print(BacsBankTransfer.to_json())

# convert the object into a dict
bacs_bank_transfer_dict = bacs_bank_transfer_instance.to_dict()
# create an instance of BacsBankTransfer from a dict
bacs_bank_transfer_from_dict = BacsBankTransfer.from_dict(bacs_bank_transfer_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


