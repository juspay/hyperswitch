# LocalBankTransferAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_code** | **str** | Partially masked bank code | [optional] 

## Example

```python
from hyperswitch.models.local_bank_transfer_additional_data import LocalBankTransferAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of LocalBankTransferAdditionalData from a JSON string
local_bank_transfer_additional_data_instance = LocalBankTransferAdditionalData.from_json(json)
# print the JSON string representation of the object
print(LocalBankTransferAdditionalData.to_json())

# convert the object into a dict
local_bank_transfer_additional_data_dict = local_bank_transfer_additional_data_instance.to_dict()
# create an instance of LocalBankTransferAdditionalData from a dict
local_bank_transfer_additional_data_from_dict = LocalBankTransferAdditionalData.from_dict(local_bank_transfer_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


