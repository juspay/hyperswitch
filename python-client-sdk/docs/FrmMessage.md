# FrmMessage

frm message is an object sent inside the payments response...when frm is invoked, its value is Some(...), else its None

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**frm_name** | **str** |  | 
**frm_transaction_id** | **str** |  | [optional] 
**frm_transaction_type** | **str** |  | [optional] 
**frm_status** | **str** |  | [optional] 
**frm_score** | **int** |  | [optional] 
**frm_reason** | **object** |  | [optional] 
**frm_error** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.frm_message import FrmMessage

# TODO update the JSON string below
json = "{}"
# create an instance of FrmMessage from a JSON string
frm_message_instance = FrmMessage.from_json(json)
# print the JSON string representation of the object
print(FrmMessage.to_json())

# convert the object into a dict
frm_message_dict = frm_message_instance.to_dict()
# create an instance of FrmMessage from a dict
frm_message_from_dict = FrmMessage.from_dict(frm_message_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


