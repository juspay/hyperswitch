# TimeRange

A type representing a range of time for filtering, including a mandatory start time and an optional end time.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**start_time** | **datetime** | The start time to filter payments list or to get list of filters. To get list of filters start time is needed to be passed | 
**end_time** | **datetime** | The end time to filter payments list or to get list of filters. If not passed the default time is now | [optional] 

## Example

```python
from hyperswitch.models.time_range import TimeRange

# TODO update the JSON string below
json = "{}"
# create an instance of TimeRange from a JSON string
time_range_instance = TimeRange.from_json(json)
# print the JSON string representation of the object
print(TimeRange.to_json())

# convert the object into a dict
time_range_dict = time_range_instance.to_dict()
# create an instance of TimeRange from a dict
time_range_from_dict = TimeRange.from_dict(time_range_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


