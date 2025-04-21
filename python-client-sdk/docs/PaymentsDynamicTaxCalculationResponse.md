# PaymentsDynamicTaxCalculationResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_id** | **str** | The identifier for the payment | 
**net_amount** | **int** | This Unit struct represents MinorUnit in which core amount works | 
**order_tax_amount** | **int** | This Unit struct represents MinorUnit in which core amount works | [optional] 
**shipping_cost** | **int** | This Unit struct represents MinorUnit in which core amount works | [optional] 
**display_amount** | [**DisplayAmountOnSdk**](DisplayAmountOnSdk.md) |  | 

## Example

```python
from hyperswitch.models.payments_dynamic_tax_calculation_response import PaymentsDynamicTaxCalculationResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsDynamicTaxCalculationResponse from a JSON string
payments_dynamic_tax_calculation_response_instance = PaymentsDynamicTaxCalculationResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentsDynamicTaxCalculationResponse.to_json())

# convert the object into a dict
payments_dynamic_tax_calculation_response_dict = payments_dynamic_tax_calculation_response_instance.to_dict()
# create an instance of PaymentsDynamicTaxCalculationResponse from a dict
payments_dynamic_tax_calculation_response_from_dict = PaymentsDynamicTaxCalculationResponse.from_dict(payments_dynamic_tax_calculation_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


