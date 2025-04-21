# Priority


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**data** | [**List[RoutableConnectorChoice]**](RoutableConnectorChoice.md) |  | 

## Example

```python
from hyperswitch.models.priority import Priority

# TODO update the JSON string below
json = "{}"
# create an instance of Priority from a JSON string
priority_instance = Priority.from_json(json)
# print the JSON string representation of the object
print(Priority.to_json())

# convert the object into a dict
priority_dict = priority_instance.to_dict()
# create an instance of Priority from a dict
priority_from_dict = Priority.from_dict(priority_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


