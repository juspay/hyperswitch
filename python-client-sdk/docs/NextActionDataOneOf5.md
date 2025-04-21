# NextActionDataOneOf5

Contains the download url and the reference number for transaction

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**voucher_details** | **str** |  | 
**type** | **str** |  | 

## Example

```python
from hyperswitch.models.next_action_data_one_of5 import NextActionDataOneOf5

# TODO update the JSON string below
json = "{}"
# create an instance of NextActionDataOneOf5 from a JSON string
next_action_data_one_of5_instance = NextActionDataOneOf5.from_json(json)
# print the JSON string representation of the object
print(NextActionDataOneOf5.to_json())

# convert the object into a dict
next_action_data_one_of5_dict = next_action_data_one_of5_instance.to_dict()
# create an instance of NextActionDataOneOf5 from a dict
next_action_data_one_of5_from_dict = NextActionDataOneOf5.from_dict(next_action_data_one_of5_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


