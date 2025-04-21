# PaymentMethodListResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**redirect_url** | **str** | Redirect URL of the merchant | [optional] 
**currency** | [**Currency**](Currency.md) |  | 
**payment_methods** | [**List[ResponsePaymentMethodsEnabled]**](ResponsePaymentMethodsEnabled.md) | Information about the payment method | 
**mandate_payment** | [**MandateType**](MandateType.md) |  | 
**merchant_name** | **str** |  | [optional] 
**show_surcharge_breakup_screen** | **bool** | flag to indicate if surcharge and tax breakup screen should be shown or not | 
**payment_type** | [**PaymentType**](PaymentType.md) |  | [optional] 
**request_external_three_ds_authentication** | **bool** | flag to indicate whether to perform external 3ds authentication | 
**collect_shipping_details_from_wallets** | **bool** | flag that indicates whether to collect shipping details from wallets or from the customer | [optional] 
**collect_billing_details_from_wallets** | **bool** | flag that indicates whether to collect billing details from wallets or from the customer | [optional] 
**is_tax_calculation_enabled** | **bool** | flag that indicates whether to calculate tax on the order amount | 

## Example

```python
from hyperswitch.models.payment_method_list_response import PaymentMethodListResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodListResponse from a JSON string
payment_method_list_response_instance = PaymentMethodListResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodListResponse.to_json())

# convert the object into a dict
payment_method_list_response_dict = payment_method_list_response_instance.to_dict()
# create an instance of PaymentMethodListResponse from a dict
payment_method_list_response_from_dict = PaymentMethodListResponse.from_dict(payment_method_list_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


