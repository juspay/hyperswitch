# PaymentMethodCreate


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | [optional] 
**payment_method_issuer** | **str** | The name of the bank/ provider issuing the payment method to the end user | [optional] 
**payment_method_issuer_code** | [**PaymentMethodIssuerCode**](PaymentMethodIssuerCode.md) |  | [optional] 
**card** | [**CardDetail**](CardDetail.md) |  | [optional] 
**metadata** | **object** | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional] 
**customer_id** | **str** | The unique identifier of the customer. | [optional] 
**card_network** | **str** | The card network | [optional] 
**bank_transfer** | [**Bank**](Bank.md) |  | [optional] 
**wallet** | [**Wallet**](Wallet.md) |  | [optional] 
**client_secret** | **str** | For Client based calls, SDK will use the client_secret in order to call /payment_methods Client secret will be generated whenever a new payment method is created | [optional] 
**payment_method_data** | [**PaymentMethodCreateData**](PaymentMethodCreateData.md) |  | [optional] 
**billing** | [**Address**](Address.md) |  | [optional] 

## Example

```python
from hyperswitch.models.payment_method_create import PaymentMethodCreate

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodCreate from a JSON string
payment_method_create_instance = PaymentMethodCreate.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodCreate.to_json())

# convert the object into a dict
payment_method_create_dict = payment_method_create_instance.to_dict()
# create an instance of PaymentMethodCreate from a dict
payment_method_create_from_dict = PaymentMethodCreate.from_dict(payment_method_create_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


