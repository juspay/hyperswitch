# ThreeDsMethodDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**three_ds_method_data_submission** | **bool** | Whether ThreeDS method data submission is required | 
**three_ds_method_data** | **str** | ThreeDS method data | [optional] 
**three_ds_method_url** | **str** | ThreeDS method url | [optional] 
**three_ds_method_key** | **str** |  | 

## Example

```python
from hyperswitch.models.three_ds_method_data_one_of import ThreeDsMethodDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of ThreeDsMethodDataOneOf from a JSON string
three_ds_method_data_one_of_instance = ThreeDsMethodDataOneOf.from_json(json)
# print the JSON string representation of the object
print(ThreeDsMethodDataOneOf.to_json())

# convert the object into a dict
three_ds_method_data_one_of_dict = three_ds_method_data_one_of_instance.to_dict()
# create an instance of ThreeDsMethodDataOneOf from a dict
three_ds_method_data_one_of_from_dict = ThreeDsMethodDataOneOf.from_dict(three_ds_method_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


