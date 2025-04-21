# ValueTypeOneOf4


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**value** | **List[int]** | Represents an array of numbers. This is basically used for \&quot;one of the given numbers\&quot; operations eg: payment.method.amount &#x3D; (1, 2, 3) | 

## Example

```python
from hyperswitch.models.value_type_one_of4 import ValueTypeOneOf4

# TODO update the JSON string below
json = "{}"
# create an instance of ValueTypeOneOf4 from a JSON string
value_type_one_of4_instance = ValueTypeOneOf4.from_json(json)
# print the JSON string representation of the object
print(ValueTypeOneOf4.to_json())

# convert the object into a dict
value_type_one_of4_dict = value_type_one_of4_instance.to_dict()
# create an instance of ValueTypeOneOf4 from a dict
value_type_one_of4_from_dict = ValueTypeOneOf4.from_dict(value_type_one_of4_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


