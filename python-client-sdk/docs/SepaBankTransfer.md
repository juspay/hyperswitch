# SepaBankTransfer


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_name** | **str** | Bank name | [optional] 
**bank_country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**bank_city** | **str** | Bank city | [optional] 
**iban** | **str** | International Bank Account Number (iban) - used in many countries for identifying a bank along with it&#39;s customer. | 
**bic** | **str** | [8 / 11 digits] Bank Identifier Code (bic) / Swift Code - used in many countries for identifying a bank and it&#39;s branches | 

## Example

```python
from hyperswitch.models.sepa_bank_transfer import SepaBankTransfer

# TODO update the JSON string below
json = "{}"
# create an instance of SepaBankTransfer from a JSON string
sepa_bank_transfer_instance = SepaBankTransfer.from_json(json)
# print the JSON string representation of the object
print(SepaBankTransfer.to_json())

# convert the object into a dict
sepa_bank_transfer_dict = sepa_bank_transfer_instance.to_dict()
# create an instance of SepaBankTransfer from a dict
sepa_bank_transfer_from_dict = SepaBankTransfer.from_dict(sepa_bank_transfer_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


