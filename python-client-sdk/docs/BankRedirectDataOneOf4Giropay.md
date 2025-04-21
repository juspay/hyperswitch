# BankRedirectDataOneOf4Giropay


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_details** | [**BankRedirectBilling**](BankRedirectBilling.md) |  | [optional] 
**bank_account_bic** | **str** | Bank account bic code | [optional] 
**bank_account_iban** | **str** | Bank account iban | [optional] 
**country** | [**CountryAlpha2**](CountryAlpha2.md) |  | 

## Example

```python
from hyperswitch.models.bank_redirect_data_one_of4_giropay import BankRedirectDataOneOf4Giropay

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectDataOneOf4Giropay from a JSON string
bank_redirect_data_one_of4_giropay_instance = BankRedirectDataOneOf4Giropay.from_json(json)
# print the JSON string representation of the object
print(BankRedirectDataOneOf4Giropay.to_json())

# convert the object into a dict
bank_redirect_data_one_of4_giropay_dict = bank_redirect_data_one_of4_giropay_instance.to_dict()
# create an instance of BankRedirectDataOneOf4Giropay from a dict
bank_redirect_data_one_of4_giropay_from_dict = BankRedirectDataOneOf4Giropay.from_dict(bank_redirect_data_one_of4_giropay_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


