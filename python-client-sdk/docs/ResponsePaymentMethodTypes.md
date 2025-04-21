# ResponsePaymentMethodTypes


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | 
**payment_experience** | [**List[PaymentExperienceTypes]**](PaymentExperienceTypes.md) | The list of payment experiences enabled, if applicable for a payment method type | [optional] 
**card_networks** | [**List[CardNetworkTypes]**](CardNetworkTypes.md) | The list of card networks enabled, if applicable for a payment method type | [optional] 
**bank_names** | [**List[BankCodeResponse]**](BankCodeResponse.md) | The list of banks enabled, if applicable for a payment method type | [optional] 
**bank_debits** | [**BankDebitTypes**](BankDebitTypes.md) |  | [optional] 
**bank_transfers** | [**BankTransferTypes**](BankTransferTypes.md) |  | [optional] 
**required_fields** | [**Dict[str, RequiredFieldInfo]**](RequiredFieldInfo.md) | Required fields for the payment_method_type. | [optional] 
**surcharge_details** | [**SurchargeDetailsResponse**](SurchargeDetailsResponse.md) |  | [optional] 
**pm_auth_connector** | **str** | auth service connector label for this payment method type, if exists | [optional] 

## Example

```python
from hyperswitch.models.response_payment_method_types import ResponsePaymentMethodTypes

# TODO update the JSON string below
json = "{}"
# create an instance of ResponsePaymentMethodTypes from a JSON string
response_payment_method_types_instance = ResponsePaymentMethodTypes.from_json(json)
# print the JSON string representation of the object
print(ResponsePaymentMethodTypes.to_json())

# convert the object into a dict
response_payment_method_types_dict = response_payment_method_types_instance.to_dict()
# create an instance of ResponsePaymentMethodTypes from a dict
response_payment_method_types_from_dict = ResponsePaymentMethodTypes.from_dict(response_payment_method_types_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


