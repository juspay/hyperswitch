# PollResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**poll_id** | **str** | The poll id | 
**status** | [**PollStatus**](PollStatus.md) |  | 

## Example

```python
from hyperswitch.models.poll_response import PollResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PollResponse from a JSON string
poll_response_instance = PollResponse.from_json(json)
# print the JSON string representation of the object
print(PollResponse.to_json())

# convert the object into a dict
poll_response_dict = poll_response_instance.to_dict()
# create an instance of PollResponse from a dict
poll_response_from_dict = PollResponse.from_dict(poll_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


