# TokenizePaymentMethodRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_cvc** | **str** | The CVC number for the card | [optional] 

## Example

```python
from hyperswitch.models.tokenize_payment_method_request import TokenizePaymentMethodRequest

# TODO update the JSON string below
json = "{}"
# create an instance of TokenizePaymentMethodRequest from a JSON string
tokenize_payment_method_request_instance = TokenizePaymentMethodRequest.from_json(json)
# print the JSON string representation of the object
print(TokenizePaymentMethodRequest.to_json())

# convert the object into a dict
tokenize_payment_method_request_dict = tokenize_payment_method_request_instance.to_dict()
# create an instance of TokenizePaymentMethodRequest from a dict
tokenize_payment_method_request_from_dict = TokenizePaymentMethodRequest.from_dict(tokenize_payment_method_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


