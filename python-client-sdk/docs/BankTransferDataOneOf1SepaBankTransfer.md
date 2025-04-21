# BankTransferDataOneOf1SepaBankTransfer


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_details** | [**SepaAndBacsBillingDetails**](SepaAndBacsBillingDetails.md) |  | [optional] 
**country** | [**CountryAlpha2**](CountryAlpha2.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_data_one_of1_sepa_bank_transfer import BankTransferDataOneOf1SepaBankTransfer

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferDataOneOf1SepaBankTransfer from a JSON string
bank_transfer_data_one_of1_sepa_bank_transfer_instance = BankTransferDataOneOf1SepaBankTransfer.from_json(json)
# print the JSON string representation of the object
print(BankTransferDataOneOf1SepaBankTransfer.to_json())

# convert the object into a dict
bank_transfer_data_one_of1_sepa_bank_transfer_dict = bank_transfer_data_one_of1_sepa_bank_transfer_instance.to_dict()
# create an instance of BankTransferDataOneOf1SepaBankTransfer from a dict
bank_transfer_data_one_of1_sepa_bank_transfer_from_dict = BankTransferDataOneOf1SepaBankTransfer.from_dict(bank_transfer_data_one_of1_sepa_bank_transfer_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


