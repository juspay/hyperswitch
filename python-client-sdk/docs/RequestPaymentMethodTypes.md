# RequestPaymentMethodTypes


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | 
**payment_experience** | [**PaymentExperience**](PaymentExperience.md) |  | [optional] 
**card_networks** | [**List[CardNetwork]**](CardNetwork.md) |  | [optional] 
**accepted_currencies** | [**AcceptedCurrencies**](AcceptedCurrencies.md) |  | [optional] 
**accepted_countries** | [**AcceptedCountries**](AcceptedCountries.md) |  | [optional] 
**minimum_amount** | **int** | This Unit struct represents MinorUnit in which core amount works | [optional] 
**maximum_amount** | **int** | This Unit struct represents MinorUnit in which core amount works | [optional] 
**recurring_enabled** | **bool** | Boolean to enable recurring payments / mandates. Default is true. | [default to True]
**installment_payment_enabled** | **bool** | Boolean to enable installment / EMI / BNPL payments. Default is true. | [default to True]

## Example

```python
from hyperswitch.models.request_payment_method_types import RequestPaymentMethodTypes

# TODO update the JSON string below
json = "{}"
# create an instance of RequestPaymentMethodTypes from a JSON string
request_payment_method_types_instance = RequestPaymentMethodTypes.from_json(json)
# print the JSON string representation of the object
print(RequestPaymentMethodTypes.to_json())

# convert the object into a dict
request_payment_method_types_dict = request_payment_method_types_instance.to_dict()
# create an instance of RequestPaymentMethodTypes from a dict
request_payment_method_types_from_dict = RequestPaymentMethodTypes.from_dict(request_payment_method_types_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


