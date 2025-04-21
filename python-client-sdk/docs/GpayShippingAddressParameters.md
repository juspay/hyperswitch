# GpayShippingAddressParameters


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**phone_number_required** | **bool** | Is shipping phone number required | 

## Example

```python
from hyperswitch.models.gpay_shipping_address_parameters import GpayShippingAddressParameters

# TODO update the JSON string below
json = "{}"
# create an instance of GpayShippingAddressParameters from a JSON string
gpay_shipping_address_parameters_instance = GpayShippingAddressParameters.from_json(json)
# print the JSON string representation of the object
print(GpayShippingAddressParameters.to_json())

# convert the object into a dict
gpay_shipping_address_parameters_dict = gpay_shipping_address_parameters_instance.to_dict()
# create an instance of GpayShippingAddressParameters from a dict
gpay_shipping_address_parameters_from_dict = GpayShippingAddressParameters.from_dict(gpay_shipping_address_parameters_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


