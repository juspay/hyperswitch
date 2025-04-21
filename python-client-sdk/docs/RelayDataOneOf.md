# RelayDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**refund** | [**RelayRefundRequestData**](RelayRefundRequestData.md) |  | 

## Example

```python
from hyperswitch.models.relay_data_one_of import RelayDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of RelayDataOneOf from a JSON string
relay_data_one_of_instance = RelayDataOneOf.from_json(json)
# print the JSON string representation of the object
print(RelayDataOneOf.to_json())

# convert the object into a dict
relay_data_one_of_dict = relay_data_one_of_instance.to_dict()
# create an instance of RelayDataOneOf from a dict
relay_data_one_of_from_dict = RelayDataOneOf.from_dict(relay_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


