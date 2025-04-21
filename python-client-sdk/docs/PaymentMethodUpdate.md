# PaymentMethodUpdate


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardDetailUpdate**](CardDetailUpdate.md) |  | [optional] 
**client_secret** | **str** | This is a 15 minute expiry token which shall be used from the client to authenticate and perform sessions from the SDK | [optional] 

## Example

```python
from hyperswitch.models.payment_method_update import PaymentMethodUpdate

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodUpdate from a JSON string
payment_method_update_instance = PaymentMethodUpdate.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodUpdate.to_json())

# convert the object into a dict
payment_method_update_dict = payment_method_update_instance.to_dict()
# create an instance of PaymentMethodUpdate from a dict
payment_method_update_from_dict = PaymentMethodUpdate.from_dict(payment_method_update_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


