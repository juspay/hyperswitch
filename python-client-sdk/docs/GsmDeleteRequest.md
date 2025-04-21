# GsmDeleteRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector** | **str** | The connector through which payment has gone through | 
**flow** | **str** | The flow in which the code and message occurred for a connector | 
**sub_flow** | **str** | The sub_flow in which the code and message occurred  for a connector | 
**code** | **str** | code received from the connector | 
**message** | **str** | message received from the connector | 

## Example

```python
from hyperswitch.models.gsm_delete_request import GsmDeleteRequest

# TODO update the JSON string below
json = "{}"
# create an instance of GsmDeleteRequest from a JSON string
gsm_delete_request_instance = GsmDeleteRequest.from_json(json)
# print the JSON string representation of the object
print(GsmDeleteRequest.to_json())

# convert the object into a dict
gsm_delete_request_dict = gsm_delete_request_instance.to_dict()
# create an instance of GsmDeleteRequest from a dict
gsm_delete_request_from_dict = GsmDeleteRequest.from_dict(gsm_delete_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


