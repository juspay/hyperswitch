# GsmUpdateRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector** | **str** | The connector through which payment has gone through | 
**flow** | **str** | The flow in which the code and message occurred for a connector | 
**sub_flow** | **str** | The sub_flow in which the code and message occurred  for a connector | 
**code** | **str** | code received from the connector | 
**message** | **str** | message received from the connector | 
**status** | **str** | status provided by the router | [optional] 
**router_error** | **str** | optional error provided by the router | [optional] 
**decision** | [**GsmDecision**](GsmDecision.md) |  | [optional] 
**step_up_possible** | **bool** | indicates if step_up retry is possible | [optional] 
**unified_code** | **str** | error code unified across the connectors | [optional] 
**unified_message** | **str** | error message unified across the connectors | [optional] 
**error_category** | [**ErrorCategory**](ErrorCategory.md) |  | [optional] 
**clear_pan_possible** | **bool** | indicates if retry with pan is possible | [optional] 

## Example

```python
from hyperswitch.models.gsm_update_request import GsmUpdateRequest

# TODO update the JSON string below
json = "{}"
# create an instance of GsmUpdateRequest from a JSON string
gsm_update_request_instance = GsmUpdateRequest.from_json(json)
# print the JSON string representation of the object
print(GsmUpdateRequest.to_json())

# convert the object into a dict
gsm_update_request_dict = gsm_update_request_instance.to_dict()
# create an instance of GsmUpdateRequest from a dict
gsm_update_request_from_dict = GsmUpdateRequest.from_dict(gsm_update_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


