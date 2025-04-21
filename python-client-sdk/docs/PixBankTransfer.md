# PixBankTransfer


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_name** | **str** | Bank name | [optional] 
**bank_branch** | **str** | Bank branch | [optional] 
**bank_account_number** | **str** | Bank account number is an unique identifier assigned by a bank to a customer. | 
**pix_key** | **str** | Unique key for pix customer | 
**tax_id** | **str** | Individual taxpayer identification number | [optional] 

## Example

```python
from hyperswitch.models.pix_bank_transfer import PixBankTransfer

# TODO update the JSON string below
json = "{}"
# create an instance of PixBankTransfer from a JSON string
pix_bank_transfer_instance = PixBankTransfer.from_json(json)
# print the JSON string representation of the object
print(PixBankTransfer.to_json())

# convert the object into a dict
pix_bank_transfer_dict = pix_bank_transfer_instance.to_dict()
# create an instance of PixBankTransfer from a dict
pix_bank_transfer_from_dict = PixBankTransfer.from_dict(pix_bank_transfer_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


