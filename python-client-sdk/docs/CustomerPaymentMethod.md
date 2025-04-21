# CustomerPaymentMethod


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_token** | **str** | Token for payment method in temporary card locker which gets refreshed often | 
**payment_method_id** | **str** | The unique identifier of the customer. | 
**customer_id** | **str** | The unique identifier of the customer. | 
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | [optional] 
**payment_method_issuer** | **str** | The name of the bank/ provider issuing the payment method to the end user | [optional] 
**payment_method_issuer_code** | [**PaymentMethodIssuerCode**](PaymentMethodIssuerCode.md) |  | [optional] 
**recurring_enabled** | **bool** | Indicates whether the payment method is eligible for recurring payments | 
**installment_payment_enabled** | **bool** | Indicates whether the payment method is eligible for installment payments | 
**payment_experience** | [**List[PaymentExperience]**](PaymentExperience.md) | Type of payment experience enabled with the connector | [optional] 
**card** | [**CardDetailFromLocker**](CardDetailFromLocker.md) |  | [optional] 
**metadata** | **object** | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional] 
**created** | **datetime** | A timestamp (ISO 8601 code) that determines when the payment method was created | [optional] 
**bank_transfer** | [**Bank**](Bank.md) |  | [optional] 
**bank** | [**MaskedBankDetails**](MaskedBankDetails.md) |  | [optional] 
**surcharge_details** | [**SurchargeDetailsResponse**](SurchargeDetailsResponse.md) |  | [optional] 
**requires_cvv** | **bool** | Whether this payment method requires CVV to be collected | 
**last_used_at** | **datetime** | A timestamp (ISO 8601 code) that determines when the payment method was last used | [optional] 
**default_payment_method_set** | **bool** | Indicates if the payment method has been set to default or not | 
**billing** | [**Address**](Address.md) |  | [optional] 

## Example

```python
from hyperswitch.models.customer_payment_method import CustomerPaymentMethod

# TODO update the JSON string below
json = "{}"
# create an instance of CustomerPaymentMethod from a JSON string
customer_payment_method_instance = CustomerPaymentMethod.from_json(json)
# print the JSON string representation of the object
print(CustomerPaymentMethod.to_json())

# convert the object into a dict
customer_payment_method_dict = customer_payment_method_instance.to_dict()
# create an instance of CustomerPaymentMethod from a dict
customer_payment_method_from_dict = CustomerPaymentMethod.from_dict(customer_payment_method_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


