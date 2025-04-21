# GiropayBankRedirectAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bic** | **str** | Masked bank account bic code | [optional] 
**iban** | **str** | Partially masked international bank account number (iban) for SEPA | [optional] 
**country** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 

## Example

```python
from hyperswitch.models.giropay_bank_redirect_additional_data import GiropayBankRedirectAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of GiropayBankRedirectAdditionalData from a JSON string
giropay_bank_redirect_additional_data_instance = GiropayBankRedirectAdditionalData.from_json(json)
# print the JSON string representation of the object
print(GiropayBankRedirectAdditionalData.to_json())

# convert the object into a dict
giropay_bank_redirect_additional_data_dict = giropay_bank_redirect_additional_data_instance.to_dict()
# create an instance of GiropayBankRedirectAdditionalData from a dict
giropay_bank_redirect_additional_data_from_dict = GiropayBankRedirectAdditionalData.from_dict(giropay_bank_redirect_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


