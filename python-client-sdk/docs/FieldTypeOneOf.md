# FieldTypeOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**user_country** | [**FieldTypeOneOfUserCountry**](FieldTypeOneOfUserCountry.md) |  | 

## Example

```python
from hyperswitch.models.field_type_one_of import FieldTypeOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of FieldTypeOneOf from a JSON string
field_type_one_of_instance = FieldTypeOneOf.from_json(json)
# print the JSON string representation of the object
print(FieldTypeOneOf.to_json())

# convert the object into a dict
field_type_one_of_dict = field_type_one_of_instance.to_dict()
# create an instance of FieldTypeOneOf from a dict
field_type_one_of_from_dict = FieldTypeOneOf.from_dict(field_type_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


