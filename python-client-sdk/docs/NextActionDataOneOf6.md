# NextActionDataOneOf6

Contains duration for displaying a wait screen, wait screen with timer is displayed by sdk

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**display_from_timestamp** | **int** |  | 
**display_to_timestamp** | **int** |  | [optional] 
**type** | **str** |  | 

## Example

```python
from hyperswitch.models.next_action_data_one_of6 import NextActionDataOneOf6

# TODO update the JSON string below
json = "{}"
# create an instance of NextActionDataOneOf6 from a JSON string
next_action_data_one_of6_instance = NextActionDataOneOf6.from_json(json)
# print the JSON string representation of the object
print(NextActionDataOneOf6.to_json())

# convert the object into a dict
next_action_data_one_of6_dict = next_action_data_one_of6_instance.to_dict()
# create an instance of NextActionDataOneOf6 from a dict
next_action_data_one_of6_from_dict = NextActionDataOneOf6.from_dict(next_action_data_one_of6_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


