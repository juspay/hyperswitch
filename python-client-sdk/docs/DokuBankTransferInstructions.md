# DokuBankTransferInstructions


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**expires_at** | **str** |  | 
**reference** | **str** |  | 
**instructions_url** | **str** |  | 

## Example

```python
from hyperswitch.models.doku_bank_transfer_instructions import DokuBankTransferInstructions

# TODO update the JSON string below
json = "{}"
# create an instance of DokuBankTransferInstructions from a JSON string
doku_bank_transfer_instructions_instance = DokuBankTransferInstructions.from_json(json)
# print the JSON string representation of the object
print(DokuBankTransferInstructions.to_json())

# convert the object into a dict
doku_bank_transfer_instructions_dict = doku_bank_transfer_instructions_instance.to_dict()
# create an instance of DokuBankTransferInstructions from a dict
doku_bank_transfer_instructions_from_dict = DokuBankTransferInstructions.from_dict(doku_bank_transfer_instructions_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


