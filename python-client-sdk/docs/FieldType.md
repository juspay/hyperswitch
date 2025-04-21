# FieldType

Possible field type of required fields in payment_method_data

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**user_country** | [**FieldTypeOneOfUserCountry**](FieldTypeOneOfUserCountry.md) |  | 
**user_currency** | [**FieldTypeOneOfUserCountry**](FieldTypeOneOfUserCountry.md) |  | 
**user_address_country** | [**FieldTypeOneOfUserCountry**](FieldTypeOneOfUserCountry.md) |  | 
**user_shipping_address_country** | [**FieldTypeOneOfUserCountry**](FieldTypeOneOfUserCountry.md) |  | 
**drop_down** | [**FieldTypeOneOfUserCountry**](FieldTypeOneOfUserCountry.md) |  | 
**language_preference** | [**FieldTypeOneOfUserCountry**](FieldTypeOneOfUserCountry.md) |  | 

## Example

```python
from hyperswitch.models.field_type import FieldType

# TODO update the JSON string below
json = "{}"
# create an instance of FieldType from a JSON string
field_type_instance = FieldType.from_json(json)
# print the JSON string representation of the object
print(FieldType.to_json())

# convert the object into a dict
field_type_dict = field_type_instance.to_dict()
# create an instance of FieldType from a dict
field_type_from_dict = FieldType.from_dict(field_type_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


