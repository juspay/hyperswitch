# PaymentMethodSpecificFeatures


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**three_ds** | [**FeatureStatus**](FeatureStatus.md) |  | 
**no_three_ds** | [**FeatureStatus**](FeatureStatus.md) |  | 
**supported_card_networks** | [**List[CardNetwork]**](CardNetwork.md) | List of supported card networks | 

## Example

```python
from hyperswitch.models.payment_method_specific_features import PaymentMethodSpecificFeatures

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodSpecificFeatures from a JSON string
payment_method_specific_features_instance = PaymentMethodSpecificFeatures.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodSpecificFeatures.to_json())

# convert the object into a dict
payment_method_specific_features_dict = payment_method_specific_features_instance.to_dict()
# create an instance of PaymentMethodSpecificFeatures from a dict
payment_method_specific_features_from_dict = PaymentMethodSpecificFeatures.from_dict(payment_method_specific_features_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


