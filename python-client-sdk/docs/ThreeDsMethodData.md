# ThreeDsMethodData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**three_ds_method_data_submission** | **bool** | Whether ThreeDS method data submission is required | 
**three_ds_method_data** | **str** | ThreeDS method data | [optional] 
**three_ds_method_url** | **str** | ThreeDS method url | [optional] 
**three_ds_method_key** | **str** |  | 

## Example

```python
from hyperswitch.models.three_ds_method_data import ThreeDsMethodData

# TODO update the JSON string below
json = "{}"
# create an instance of ThreeDsMethodData from a JSON string
three_ds_method_data_instance = ThreeDsMethodData.from_json(json)
# print the JSON string representation of the object
print(ThreeDsMethodData.to_json())

# convert the object into a dict
three_ds_method_data_dict = three_ds_method_data_instance.to_dict()
# create an instance of ThreeDsMethodData from a dict
three_ds_method_data_from_dict = ThreeDsMethodData.from_dict(three_ds_method_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


