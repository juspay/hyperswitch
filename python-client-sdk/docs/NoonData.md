# NoonData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**order_category** | **str** | Information about the order category that merchant wants to specify at connector level. (e.g. In Noon Payments it can take values like \&quot;pay\&quot;, \&quot;food\&quot;, or any other custom string set by the merchant in Noon&#39;s Dashboard) | [optional] 

## Example

```python
from hyperswitch.models.noon_data import NoonData

# TODO update the JSON string below
json = "{}"
# create an instance of NoonData from a JSON string
noon_data_instance = NoonData.from_json(json)
# print the JSON string representation of the object
print(NoonData.to_json())

# convert the object into a dict
noon_data_dict = noon_data_instance.to_dict()
# create an instance of NoonData from a dict
noon_data_from_dict = NoonData.from_dict(noon_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


