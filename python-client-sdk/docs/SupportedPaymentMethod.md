# SupportedPaymentMethod


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**three_ds** | [**FeatureStatus**](FeatureStatus.md) |  | 
**no_three_ds** | [**FeatureStatus**](FeatureStatus.md) |  | 
**supported_card_networks** | [**List[CardNetwork]**](CardNetwork.md) | List of supported card networks | 
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | 
**payment_method_type_display_name** | **str** | The display name of the payment method type | 
**mandates** | [**FeatureStatus**](FeatureStatus.md) |  | 
**refunds** | [**FeatureStatus**](FeatureStatus.md) |  | 
**supported_capture_methods** | [**List[CaptureMethod]**](CaptureMethod.md) | List of supported capture methods supported by the payment method type | 
**supported_countries** | [**List[CountryAlpha3]**](CountryAlpha3.md) | List of countries supported by the payment method type via the connector | [optional] 
**supported_currencies** | [**List[Currency]**](Currency.md) | List of currencies supported by the payment method type via the connector | [optional] 

## Example

```python
from hyperswitch.models.supported_payment_method import SupportedPaymentMethod

# TODO update the JSON string below
json = "{}"
# create an instance of SupportedPaymentMethod from a JSON string
supported_payment_method_instance = SupportedPaymentMethod.from_json(json)
# print the JSON string representation of the object
print(SupportedPaymentMethod.to_json())

# convert the object into a dict
supported_payment_method_dict = supported_payment_method_instance.to_dict()
# create an instance of SupportedPaymentMethod from a dict
supported_payment_method_from_dict = SupportedPaymentMethod.from_dict(supported_payment_method_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


