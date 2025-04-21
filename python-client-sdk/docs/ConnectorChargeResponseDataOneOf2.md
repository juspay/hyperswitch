# ConnectorChargeResponseDataOneOf2


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**xendit_split_payment** | [**XenditChargeResponseData**](XenditChargeResponseData.md) |  | 

## Example

```python
from hyperswitch.models.connector_charge_response_data_one_of2 import ConnectorChargeResponseDataOneOf2

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorChargeResponseDataOneOf2 from a JSON string
connector_charge_response_data_one_of2_instance = ConnectorChargeResponseDataOneOf2.from_json(json)
# print the JSON string representation of the object
print(ConnectorChargeResponseDataOneOf2.to_json())

# convert the object into a dict
connector_charge_response_data_one_of2_dict = connector_charge_response_data_one_of2_instance.to_dict()
# create an instance of ConnectorChargeResponseDataOneOf2 from a dict
connector_charge_response_data_one_of2_from_dict = ConnectorChargeResponseDataOneOf2.from_dict(connector_charge_response_data_one_of2_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


