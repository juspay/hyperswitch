# GsmDeleteResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**gsm_rule_delete** | **bool** |  | 
**connector** | **str** | The connector through which payment has gone through | 
**flow** | **str** | The flow in which the code and message occurred for a connector | 
**sub_flow** | **str** | The sub_flow in which the code and message occurred  for a connector | 
**code** | **str** | code received from the connector | 

## Example

```python
from hyperswitch.models.gsm_delete_response import GsmDeleteResponse

# TODO update the JSON string below
json = "{}"
# create an instance of GsmDeleteResponse from a JSON string
gsm_delete_response_instance = GsmDeleteResponse.from_json(json)
# print the JSON string representation of the object
print(GsmDeleteResponse.to_json())

# convert the object into a dict
gsm_delete_response_dict = gsm_delete_response_instance.to_dict()
# create an instance of GsmDeleteResponse from a dict
gsm_delete_response_from_dict = GsmDeleteResponse.from_dict(gsm_delete_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


