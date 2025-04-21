# GpayTokenParameters


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**gateway** | **str** | The name of the connector | [optional] 
**gateway_merchant_id** | **str** | The merchant ID registered in the connector associated | [optional] 
**stripe_version** | **str** |  | [optional] 
**stripe_publishable_key** | **str** |  | [optional] 
**protocol_version** | **str** | The protocol version for encryption | [optional] 
**public_key** | **str** | The public key provided by the merchant | [optional] 

## Example

```python
from hyperswitch.models.gpay_token_parameters import GpayTokenParameters

# TODO update the JSON string below
json = "{}"
# create an instance of GpayTokenParameters from a JSON string
gpay_token_parameters_instance = GpayTokenParameters.from_json(json)
# print the JSON string representation of the object
print(GpayTokenParameters.to_json())

# convert the object into a dict
gpay_token_parameters_dict = gpay_token_parameters_instance.to_dict()
# create an instance of GpayTokenParameters from a dict
gpay_token_parameters_from_dict = GpayTokenParameters.from_dict(gpay_token_parameters_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


