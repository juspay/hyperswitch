# GpayBillingAddressParameters


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**phone_number_required** | **bool** | Is billing phone number required | 
**format** | [**GpayBillingAddressFormat**](GpayBillingAddressFormat.md) |  | 

## Example

```python
from hyperswitch.models.gpay_billing_address_parameters import GpayBillingAddressParameters

# TODO update the JSON string below
json = "{}"
# create an instance of GpayBillingAddressParameters from a JSON string
gpay_billing_address_parameters_instance = GpayBillingAddressParameters.from_json(json)
# print the JSON string representation of the object
print(GpayBillingAddressParameters.to_json())

# convert the object into a dict
gpay_billing_address_parameters_dict = gpay_billing_address_parameters_instance.to_dict()
# create an instance of GpayBillingAddressParameters from a dict
gpay_billing_address_parameters_from_dict = GpayBillingAddressParameters.from_dict(gpay_billing_address_parameters_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


