# TotalEventsResponse

The response body of list initial delivery attempts api call.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**events** | [**List[EventListItemResponse]**](EventListItemResponse.md) | The list of events | 
**total_count** | **int** | Count of total events | 

## Example

```python
from hyperswitch.models.total_events_response import TotalEventsResponse

# TODO update the JSON string below
json = "{}"
# create an instance of TotalEventsResponse from a JSON string
total_events_response_instance = TotalEventsResponse.from_json(json)
# print the JSON string representation of the object
print(TotalEventsResponse.to_json())

# convert the object into a dict
total_events_response_dict = total_events_response_instance.to_dict()
# create an instance of TotalEventsResponse from a dict
total_events_response_from_dict = TotalEventsResponse.from_dict(total_events_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


