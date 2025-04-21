# GpayAllowedMethodsParameters


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**allowed_auth_methods** | **List[str]** | The list of allowed auth methods (ex: 3DS, No3DS, PAN_ONLY etc) | 
**allowed_card_networks** | **List[str]** | The list of allowed card networks (ex: AMEX,JCB etc) | 
**billing_address_required** | **bool** | Is billing address required | [optional] 
**billing_address_parameters** | [**GpayBillingAddressParameters**](GpayBillingAddressParameters.md) |  | [optional] 
**assurance_details_required** | **bool** | Whether assurance details are required | [optional] 

## Example

```python
from hyperswitch.models.gpay_allowed_methods_parameters import GpayAllowedMethodsParameters

# TODO update the JSON string below
json = "{}"
# create an instance of GpayAllowedMethodsParameters from a JSON string
gpay_allowed_methods_parameters_instance = GpayAllowedMethodsParameters.from_json(json)
# print the JSON string representation of the object
print(GpayAllowedMethodsParameters.to_json())

# convert the object into a dict
gpay_allowed_methods_parameters_dict = gpay_allowed_methods_parameters_instance.to_dict()
# create an instance of GpayAllowedMethodsParameters from a dict
gpay_allowed_methods_parameters_from_dict = GpayAllowedMethodsParameters.from_dict(gpay_allowed_methods_parameters_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


