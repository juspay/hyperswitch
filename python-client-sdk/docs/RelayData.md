# RelayData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**refund** | [**RelayRefundRequestData**](RelayRefundRequestData.md) |  | 

## Example

```python
from hyperswitch.models.relay_data import RelayData

# TODO update the JSON string below
json = "{}"
# create an instance of RelayData from a JSON string
relay_data_instance = RelayData.from_json(json)
# print the JSON string representation of the object
print(RelayData.to_json())

# convert the object into a dict
relay_data_dict = relay_data_instance.to_dict()
# create an instance of RelayData from a dict
relay_data_from_dict = RelayData.from_dict(relay_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


