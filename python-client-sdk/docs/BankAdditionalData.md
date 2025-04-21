# BankAdditionalData

Masked payout method details for bank payout method

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_account_number** | **str** | Bank account&#39;s owner name | 
**bank_routing_number** | **str** | Partially masked routing number for ach bank debit payment | 
**bank_name** | **str** | Bank name | [optional] 
**bank_country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**bank_city** | **str** | Bank city | [optional] 
**bank_sort_code** | **str** | Partially masked sort code for Bacs payment method | 
**iban** | **str** | Partially masked international bank account number (iban) for SEPA | 
**bic** | **str** | [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it&#39;s branches | [optional] 
**pix_key** | **str** | Partially masked unique key for pix transfer | [optional] 
**cpf** | **str** | Partially masked CPF - CPF is a Brazilian tax identification number | [optional] 
**cnpj** | **str** | Partially masked CNPJ - CNPJ is a Brazilian company tax identification number | [optional] 

## Example

```python
from hyperswitch.models.bank_additional_data import BankAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of BankAdditionalData from a JSON string
bank_additional_data_instance = BankAdditionalData.from_json(json)
# print the JSON string representation of the object
print(BankAdditionalData.to_json())

# convert the object into a dict
bank_additional_data_dict = bank_additional_data_instance.to_dict()
# create an instance of BankAdditionalData from a dict
bank_additional_data_from_dict = BankAdditionalData.from_dict(bank_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


