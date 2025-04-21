# RequiredFieldInfo

Required fields info used while listing the payment_method_data

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**required_field** | **str** | Required field for a payment_method through a payment_method_type | 
**display_name** | **str** | Display name of the required field in the front-end | 
**field_type** | [**FieldType**](FieldType.md) |  | 
**value** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.required_field_info import RequiredFieldInfo

# TODO update the JSON string below
json = "{}"
# create an instance of RequiredFieldInfo from a JSON string
required_field_info_instance = RequiredFieldInfo.from_json(json)
# print the JSON string representation of the object
print(RequiredFieldInfo.to_json())

# convert the object into a dict
required_field_info_dict = required_field_info_instance.to_dict()
# create an instance of RequiredFieldInfo from a dict
required_field_info_from_dict = RequiredFieldInfo.from_dict(required_field_info_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


