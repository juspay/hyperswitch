# FieldTypeOneOf3


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**user_shipping_address_country** | [**FieldTypeOneOfUserCountry**](FieldTypeOneOfUserCountry.md) |  | 

## Example

```python
from hyperswitch.models.field_type_one_of3 import FieldTypeOneOf3

# TODO update the JSON string below
json = "{}"
# create an instance of FieldTypeOneOf3 from a JSON string
field_type_one_of3_instance = FieldTypeOneOf3.from_json(json)
# print the JSON string representation of the object
print(FieldTypeOneOf3.to_json())

# convert the object into a dict
field_type_one_of3_dict = field_type_one_of3_instance.to_dict()
# create an instance of FieldTypeOneOf3 from a dict
field_type_one_of3_from_dict = FieldTypeOneOf3.from_dict(field_type_one_of3_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


