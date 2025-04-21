# Bank


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_name** | **str** | Bank name | [optional] 
**bank_country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**bank_city** | **str** | Bank city | [optional] 
**bank_account_number** | **str** | Bank account number is an unique identifier assigned by a bank to a customer. | 
**bank_routing_number** | **str** | [9 digits] Routing number - used in USA for identifying a specific bank. | 
**bank_sort_code** | **str** | [6 digits] Sort Code - used in UK and Ireland for identifying a bank and it&#39;s branches. | 
**iban** | **str** | International Bank Account Number (iban) - used in many countries for identifying a bank along with it&#39;s customer. | 
**bic** | **str** | [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it&#39;s branches | 
**bank_branch** | **str** | Bank branch | [optional] 
**pix_key** | **str** | Unique key for pix customer | 
**tax_id** | **str** | Individual taxpayer identification number | [optional] 

## Example

```python
from hyperswitch.models.bank import Bank

# TODO update the JSON string below
json = "{}"
# create an instance of Bank from a JSON string
bank_instance = Bank.from_json(json)
# print the JSON string representation of the object
print(Bank.to_json())

# convert the object into a dict
bank_dict = bank_instance.to_dict()
# create an instance of Bank from a dict
bank_from_dict = Bank.from_dict(bank_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


