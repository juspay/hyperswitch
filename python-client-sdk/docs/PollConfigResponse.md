# PollConfigResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**poll_id** | **str** | Poll Id | 
**delay_in_secs** | **int** | Interval of the poll | 
**frequency** | **int** | Frequency of the poll | 

## Example

```python
from hyperswitch.models.poll_config_response import PollConfigResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PollConfigResponse from a JSON string
poll_config_response_instance = PollConfigResponse.from_json(json)
# print the JSON string representation of the object
print(PollConfigResponse.to_json())

# convert the object into a dict
poll_config_response_dict = poll_config_response_instance.to_dict()
# create an instance of PollConfigResponse from a dict
poll_config_response_from_dict = PollConfigResponse.from_dict(poll_config_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


