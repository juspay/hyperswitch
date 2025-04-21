# ConnectorChargeResponseDataOneOf1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**adyen_split_payment** | [**AdyenSplitData**](AdyenSplitData.md) |  | 

## Example

```python
from hyperswitch.models.connector_charge_response_data_one_of1 import ConnectorChargeResponseDataOneOf1

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorChargeResponseDataOneOf1 from a JSON string
connector_charge_response_data_one_of1_instance = ConnectorChargeResponseDataOneOf1.from_json(json)
# print the JSON string representation of the object
print(ConnectorChargeResponseDataOneOf1.to_json())

# convert the object into a dict
connector_charge_response_data_one_of1_dict = connector_charge_response_data_one_of1_instance.to_dict()
# create an instance of ConnectorChargeResponseDataOneOf1 from a dict
connector_charge_response_data_one_of1_from_dict = ConnectorChargeResponseDataOneOf1.from_dict(connector_charge_response_data_one_of1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


