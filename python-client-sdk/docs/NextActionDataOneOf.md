# NextActionDataOneOf

Contains the url for redirection flow

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**redirect_to_url** | **str** |  | 
**type** | **str** |  | 

## Example

```python
from hyperswitch.models.next_action_data_one_of import NextActionDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of NextActionDataOneOf from a JSON string
next_action_data_one_of_instance = NextActionDataOneOf.from_json(json)
# print the JSON string representation of the object
print(NextActionDataOneOf.to_json())

# convert the object into a dict
next_action_data_one_of_dict = next_action_data_one_of_instance.to_dict()
# create an instance of NextActionDataOneOf from a dict
next_action_data_one_of_from_dict = NextActionDataOneOf.from_dict(next_action_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


