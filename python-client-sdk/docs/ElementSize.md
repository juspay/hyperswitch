# ElementSize


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**variants** | [**SizeVariants**](SizeVariants.md) |  | 
**percentage** | **int** |  | 
**pixels** | **int** |  | 

## Example

```python
from hyperswitch.models.element_size import ElementSize

# TODO update the JSON string below
json = "{}"
# create an instance of ElementSize from a JSON string
element_size_instance = ElementSize.from_json(json)
# print the JSON string representation of the object
print(ElementSize.to_json())

# convert the object into a dict
element_size_dict = element_size_instance.to_dict()
# create an instance of ElementSize from a dict
element_size_from_dict = ElementSize.from_dict(element_size_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


