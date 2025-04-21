# PaymentsPostSessionTokensRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**client_secret** | **str** | It&#39;s a token used for client side verification. | 
**payment_method_type** | [**PaymentMethodType**](PaymentMethodType.md) |  | 
**payment_method** | [**PaymentMethod**](PaymentMethod.md) |  | 

## Example

```python
from hyperswitch.models.payments_post_session_tokens_request import PaymentsPostSessionTokensRequest

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentsPostSessionTokensRequest from a JSON string
payments_post_session_tokens_request_instance = PaymentsPostSessionTokensRequest.from_json(json)
# print the JSON string representation of the object
print(PaymentsPostSessionTokensRequest.to_json())

# convert the object into a dict
payments_post_session_tokens_request_dict = payments_post_session_tokens_request_instance.to_dict()
# create an instance of PaymentsPostSessionTokensRequest from a dict
payments_post_session_tokens_request_from_dict = PaymentsPostSessionTokensRequest.from_dict(payments_post_session_tokens_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


