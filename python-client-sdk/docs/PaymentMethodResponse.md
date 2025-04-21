# PaymentMethodResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | Unique identifier for a merchant | 
**customer_id** | **str** | The unique identifier of the customer. | [optional] 
**payment_method_id** | **str** | The unique identifier of the Payment method | 
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | [optional] 
**card** | [**CardDetailFromLocker**](CardDetailFromLocker.md) |  | [optional] 
**recurring_enabled** | **bool** | Indicates whether the payment method is eligible for recurring payments | 
**installment_payment_enabled** | **bool** | Indicates whether the payment method is eligible for installment payments | 
**payment_experience** | [**List[PaymentExperience]**](PaymentExperience.md) | Type of payment experience enabled with the connector | [optional] 
**metadata** | **object** | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional] 
**created** | **datetime** | A timestamp (ISO 8601 code) that determines when the payment method was created | [optional] 
**bank_transfer** | [**Bank**](Bank.md) |  | [optional] 
**last_used_at** | **datetime** |  | [optional] 
**client_secret** | **str** | For Client based calls | [optional] 

## Example

```python
from hyperswitch.models.payment_method_response import PaymentMethodResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodResponse from a JSON string
payment_method_response_instance = PaymentMethodResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodResponse.to_json())

# convert the object into a dict
payment_method_response_dict = payment_method_response_instance.to_dict()
# create an instance of PaymentMethodResponse from a dict
payment_method_response_from_dict = PaymentMethodResponse.from_dict(payment_method_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


