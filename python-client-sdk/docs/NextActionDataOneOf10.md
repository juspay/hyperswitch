# NextActionDataOneOf10

Contains data required to invoke hidden iframe

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**iframe_data** | [**IframeData**](IframeData.md) |  | 
**type** | **str** |  | 

## Example

```python
from hyperswitch.models.next_action_data_one_of10 import NextActionDataOneOf10

# TODO update the JSON string below
json = "{}"
# create an instance of NextActionDataOneOf10 from a JSON string
next_action_data_one_of10_instance = NextActionDataOneOf10.from_json(json)
# print the JSON string representation of the object
print(NextActionDataOneOf10.to_json())

# convert the object into a dict
next_action_data_one_of10_dict = next_action_data_one_of10_instance.to_dict()
# create an instance of NextActionDataOneOf10 from a dict
next_action_data_one_of10_from_dict = NextActionDataOneOf10.from_dict(next_action_data_one_of10_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


