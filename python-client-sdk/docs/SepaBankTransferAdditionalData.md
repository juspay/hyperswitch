# SepaBankTransferAdditionalData

Masked payout method details for sepa bank transfer payout method

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**iban** | **str** | Partially masked international bank account number (iban) for SEPA | 
**bank_name** | **str** | Bank name | [optional] 
**bank_country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**bank_city** | **str** | Bank city | [optional] 
**bic** | **str** | [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it&#39;s branches | [optional] 

## Example

```python
from hyperswitch.models.sepa_bank_transfer_additional_data import SepaBankTransferAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of SepaBankTransferAdditionalData from a JSON string
sepa_bank_transfer_additional_data_instance = SepaBankTransferAdditionalData.from_json(json)
# print the JSON string representation of the object
print(SepaBankTransferAdditionalData.to_json())

# convert the object into a dict
sepa_bank_transfer_additional_data_dict = sepa_bank_transfer_additional_data_instance.to_dict()
# create an instance of SepaBankTransferAdditionalData from a dict
sepa_bank_transfer_additional_data_from_dict = SepaBankTransferAdditionalData.from_dict(sepa_bank_transfer_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


