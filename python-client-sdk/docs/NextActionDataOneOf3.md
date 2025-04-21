# NextActionDataOneOf3

Contains url for Qr code image, this qr code has to be shown in sdk

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**image_data_url** | **str** | Hyperswitch generated image data source url | 
**display_to_timestamp** | **int** |  | [optional] 
**qr_code_url** | **str** | The url for Qr code given by the connector | 
**display_text** | **str** |  | [optional] 
**border_color** | **str** |  | [optional] 
**type** | **str** |  | 

## Example

```python
from hyperswitch.models.next_action_data_one_of3 import NextActionDataOneOf3

# TODO update the JSON string below
json = "{}"
# create an instance of NextActionDataOneOf3 from a JSON string
next_action_data_one_of3_instance = NextActionDataOneOf3.from_json(json)
# print the JSON string representation of the object
print(NextActionDataOneOf3.to_json())

# convert the object into a dict
next_action_data_one_of3_dict = next_action_data_one_of3_instance.to_dict()
# create an instance of NextActionDataOneOf3 from a dict
next_action_data_one_of3_from_dict = NextActionDataOneOf3.from_dict(next_action_data_one_of3_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


