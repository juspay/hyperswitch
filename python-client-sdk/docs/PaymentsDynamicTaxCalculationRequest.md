# PaymentsDynamicTaxCalculationRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**shipping** | [**Address**](Address.md) |  | 
**client_secret** | **str** | Client Secret | 
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | 
**session_id** | **str** | Session Id | [optional] 

## Example

```python
from hyperswitch.models.payments_dynamic_tax_calculation_request import PaymentsDynamicTaxCalculationRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsDynamicTaxCalculationRequest from a JSON string
payments_dynamic_tax_calculation_request_instance = PaymentsDynamicTaxCalculationRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsDynamicTaxCalculationRequest.to_json())

# convert the object into a dict
payments_dynamic_tax_calculation_request_dict = payments_dynamic_tax_calculation_request_instance.to_dict()
# create an instance of PaymentsDynamicTaxCalculationRequest from a dict
payments_dynamic_tax_calculation_request_from_dict = PaymentsDynamicTaxCalculationRequest.from_dict(payments_dynamic_tax_calculation_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


