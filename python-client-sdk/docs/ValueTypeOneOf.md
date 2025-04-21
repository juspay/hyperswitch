# ValueTypeOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**value** | **int** | This Unit struct represents MinorUnit in which core amount works | 

## Example

```python
from hyperswitch.models.value_type_one_of import ValueTypeOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of ValueTypeOneOf from a JSON string
value_type_one_of_instance = ValueTypeOneOf.from_json(json)
# print the JSON string representation of the object
print(ValueTypeOneOf.to_json())

# convert the object into a dict
value_type_one_of_dict = value_type_one_of_instance.to_dict()
# create an instance of ValueTypeOneOf from a dict
value_type_one_of_from_dict = ValueTypeOneOf.from_dict(value_type_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


