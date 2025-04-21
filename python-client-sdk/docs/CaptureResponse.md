# CaptureResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**capture_id** | **str** | Unique identifier for the capture | 
**status** | [**CaptureStatus**](CaptureStatus.md) |  | 
**amount** | **int** | The capture amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., | 
**currency** | [**Currency**](Currency.md) |  | [optional] 
**connector** | **str** | The connector used for the payment | 
**authorized_attempt_id** | **str** | Unique identifier for the parent attempt on which this capture is made | 
**connector_capture_id** | **str** | A unique identifier for this capture provided by the connector | [optional] 
**capture_sequence** | **int** | Sequence number of this capture, in the series of captures made for the parent attempt | 
**error_message** | **str** | If there was an error while calling the connector the error message is received here | [optional] 
**error_code** | **str** | If there was an error while calling the connectors the code is received here | [optional] 
**error_reason** | **str** | If there was an error while calling the connectors the reason is received here | [optional] 
**reference_id** | **str** | Reference to the capture at connector side | [optional] 

## Example

```python
from hyperswitch.models.capture_response import CaptureResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CaptureResponse from a JSON string
capture_response_instance = CaptureResponse.from_json(json)
# print the JSON string representation of the object
print(CaptureResponse.to_json())

# convert the object into a dict
capture_response_dict = capture_response_instance.to_dict()
# create an instance of CaptureResponse from a dict
capture_response_from_dict = CaptureResponse.from_dict(capture_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


