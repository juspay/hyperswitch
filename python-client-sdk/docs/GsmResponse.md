# GsmResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector** | **str** | The connector through which payment has gone through | 
**flow** | **str** | The flow in which the code and message occurred for a connector | 
**sub_flow** | **str** | The sub_flow in which the code and message occurred  for a connector | 
**code** | **str** | code received from the connector | 
**message** | **str** | message received from the connector | 
**status** | **str** | status provided by the router | 
**router_error** | **str** | optional error provided by the router | [optional] 
**decision** | **str** | decision to be taken for auto retries flow | 
**step_up_possible** | **bool** | indicates if step_up retry is possible | 
**unified_code** | **str** | error code unified across the connectors | [optional] 
**unified_message** | **str** | error message unified across the connectors | [optional] 
**error_category** | [**ErrorCategory**](ErrorCategory.md) |  | [optional] 
**clear_pan_possible** | **bool** | indicates if retry with pan is possible | 

## Example

```python
from hyperswitch.models.gsm_response import GsmResponse

# TODO update the JSON string below
json = "{}"
# create an instance of GsmResponse from a JSON string
gsm_response_instance = GsmResponse.from_json(json)
# print the JSON string representation of the object
print(GsmResponse.to_json())

# convert the object into a dict
gsm_response_dict = gsm_response_instance.to_dict()
# create an instance of GsmResponse from a dict
gsm_response_from_dict = GsmResponse.from_dict(gsm_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


