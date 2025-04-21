# FrmPaymentMethodType

Details of FrmPaymentMethodType are mentioned here... it should be passed in payment connector create api call, and stored in merchant_connector_table

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | 
**card_networks** | [**CardNetwork**](CardNetwork.md) |  | 
**flow** | [**FrmPreferredFlowTypes**](FrmPreferredFlowTypes.md) |  | 
**action** | [**FrmAction**](FrmAction.md) |  | 

## Example

```python
from hyperswitch.models.frm_payment_method_type import FrmPaymentMethodType

# TODO update the JSON string below
json = "{}"
# create an instance of FrmPaymentMethodType from a JSON string
frm_payment_method_type_instance = FrmPaymentMethodType.from_json(json)
# print the JSON string representation of the object
print(FrmPaymentMethodType.to_json())

# convert the object into a dict
frm_payment_method_type_dict = frm_payment_method_type_instance.to_dict()
# create an instance of FrmPaymentMethodType from a dict
frm_payment_method_type_from_dict = FrmPaymentMethodType.from_dict(frm_payment_method_type_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


