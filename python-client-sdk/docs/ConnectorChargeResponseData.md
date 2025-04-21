# ConnectorChargeResponseData

Charge Information

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**stripe_split_payment** | [**StripeChargeResponseData**](StripeChargeResponseData.md) |  | 
**adyen_split_payment** | [**AdyenSplitData**](AdyenSplitData.md) |  | 
**xendit_split_payment** | [**XenditChargeResponseData**](XenditChargeResponseData.md) |  | 

## Example

```python
from hyperswitch.models.connector_charge_response_data import ConnectorChargeResponseData

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorChargeResponseData from a JSON string
connector_charge_response_data_instance = ConnectorChargeResponseData.from_json(json)
# print the JSON string representation of the object
print(ConnectorChargeResponseData.to_json())

# convert the object into a dict
connector_charge_response_data_dict = connector_charge_response_data_instance.to_dict()
# create an instance of ConnectorChargeResponseData from a dict
connector_charge_response_data_from_dict = ConnectorChargeResponseData.from_dict(connector_charge_response_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


