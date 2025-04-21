# ConnectorChargeResponseDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**stripe_split_payment** | [**StripeChargeResponseData**](StripeChargeResponseData.md) |  | 

## Example

```python
from hyperswitch.models.connector_charge_response_data_one_of import ConnectorChargeResponseDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorChargeResponseDataOneOf from a JSON string
connector_charge_response_data_one_of_instance = ConnectorChargeResponseDataOneOf.from_json(json)
# print the JSON string representation of the object
print(ConnectorChargeResponseDataOneOf.to_json())

# convert the object into a dict
connector_charge_response_data_one_of_dict = connector_charge_response_data_one_of_instance.to_dict()
# create an instance of ConnectorChargeResponseDataOneOf from a dict
connector_charge_response_data_one_of_from_dict = ConnectorChargeResponseDataOneOf.from_dict(connector_charge_response_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


