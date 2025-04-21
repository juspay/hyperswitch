# ValueTypeOneOf6


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**value** | [**List[NumberComparison]**](NumberComparison.md) | Like a number array but can include comparisons. Useful for conditions like \&quot;500 &lt; amount &lt; 1000\&quot; eg: payment.amount &#x3D; (&gt; 500, &lt; 1000) | 

## Example

```python
from hyperswitch.models.value_type_one_of6 import ValueTypeOneOf6

# TODO update the JSON string below
json = "{}"
# create an instance of ValueTypeOneOf6 from a JSON string
value_type_one_of6_instance = ValueTypeOneOf6.from_json(json)
# print the JSON string representation of the object
print(ValueTypeOneOf6.to_json())

# convert the object into a dict
value_type_one_of6_dict = value_type_one_of6_instance.to_dict()
# create an instance of ValueTypeOneOf6 from a dict
value_type_one_of6_from_dict = ValueTypeOneOf6.from_dict(value_type_one_of6_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


