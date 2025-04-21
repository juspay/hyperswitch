# ValueType

Represents a value in the DSL

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**value** | [**List[NumberComparison]**](NumberComparison.md) | Like a number array but can include comparisons. Useful for conditions like \&quot;500 &lt; amount &lt; 1000\&quot; eg: payment.amount &#x3D; (&gt; 500, &lt; 1000) | 

## Example

```python
from hyperswitch.models.value_type import ValueType

# TODO update the JSON string below
json = "{}"
# create an instance of ValueType from a JSON string
value_type_instance = ValueType.from_json(json)
# print the JSON string representation of the object
print(ValueType.to_json())

# convert the object into a dict
value_type_dict = value_type_instance.to_dict()
# create an instance of ValueType from a dict
value_type_from_dict = ValueType.from_dict(value_type_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


