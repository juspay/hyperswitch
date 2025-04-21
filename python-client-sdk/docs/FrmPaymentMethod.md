# FrmPaymentMethod

Details of FrmPaymentMethod are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 
**payment_method_types** | [**List[FrmPaymentMethodType]**](FrmPaymentMethodType.md) | payment method types(credit, debit) that can be used in the payment. This field is deprecated. It has not been removed to provide backward compatibility. | [optional] 
**flow** | [**FrmPreferredFlowTypes**](FrmPreferredFlowTypes.md) |  | [optional] 

## Example

```python
from hyperswitch.models.frm_payment_method import FrmPaymentMethod

# TODO update the JSON string below
json = "{}"
# create an instance of FrmPaymentMethod from a JSON string
frm_payment_method_instance = FrmPaymentMethod.from_json(json)
# print the JSON string representation of the object
print(FrmPaymentMethod.to_json())

# convert the object into a dict
frm_payment_method_dict = frm_payment_method_instance.to_dict()
# create an instance of FrmPaymentMethod from a dict
frm_payment_method_from_dict = FrmPaymentMethod.from_dict(frm_payment_method_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


