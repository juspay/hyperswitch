# SepaBankTransferInstructions


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**account_holder_name** | **str** |  | 
**bic** | **str** |  | 
**country** | **str** |  | 
**iban** | **str** |  | 
**reference** | **str** |  | 

## Example

```python
from hyperswitch.models.sepa_bank_transfer_instructions import SepaBankTransferInstructions

# TODO update the JSON string below
json = "{}"
# create an instance of SepaBankTransferInstructions from a JSON string
sepa_bank_transfer_instructions_instance = SepaBankTransferInstructions.from_json(json)
# print the JSON string representation of the object
print(SepaBankTransferInstructions.to_json())

# convert the object into a dict
sepa_bank_transfer_instructions_dict = sepa_bank_transfer_instructions_instance.to_dict()
# create an instance of SepaBankTransferInstructions from a dict
sepa_bank_transfer_instructions_from_dict = SepaBankTransferInstructions.from_dict(sepa_bank_transfer_instructions_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


