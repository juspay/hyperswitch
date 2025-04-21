# SepaBankDebitAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**iban** | **str** | Partially masked international bank account number (iban) for SEPA | 
**bank_account_holder_name** | **str** | Bank account&#39;s owner name | [optional] 

## Example

```python
from hyperswitch.models.sepa_bank_debit_additional_data import SepaBankDebitAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of SepaBankDebitAdditionalData from a JSON string
sepa_bank_debit_additional_data_instance = SepaBankDebitAdditionalData.from_json(json)
# print the JSON string representation of the object
print(SepaBankDebitAdditionalData.to_json())

# convert the object into a dict
sepa_bank_debit_additional_data_dict = sepa_bank_debit_additional_data_instance.to_dict()
# create an instance of SepaBankDebitAdditionalData from a dict
sepa_bank_debit_additional_data_from_dict = SepaBankDebitAdditionalData.from_dict(sepa_bank_debit_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


