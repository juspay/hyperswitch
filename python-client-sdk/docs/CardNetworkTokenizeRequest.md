# CardNetworkTokenizeRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**TokenizeCardRequest**](TokenizeCardRequest.md) |  | 
**existing_payment_method** | [**TokenizePaymentMethodRequest**](TokenizePaymentMethodRequest.md) |  | 
**merchant_id** | **str** | Merchant ID associated with the tokenization request | 
**customer** | [**CustomerDetails**](CustomerDetails.md) |  | 
**billing** | [**Address**](Address.md) |  | [optional] 
**metadata** | **object** | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional] 
**payment_method_issuer** | **str** | The name of the bank/ provider issuing the payment method to the end user | [optional] 

## Example

```python
from hyperswitch.models.card_network_tokenize_request import CardNetworkTokenizeRequest

# TODO update the JSON string below
json = "{}"
# create an instance of CardNetworkTokenizeRequest from a JSON string
card_network_tokenize_request_instance = CardNetworkTokenizeRequest.from_json(json)
# print the JSON string representation of the object
print(CardNetworkTokenizeRequest.to_json())

# convert the object into a dict
card_network_tokenize_request_dict = card_network_tokenize_request_instance.to_dict()
# create an instance of CardNetworkTokenizeRequest from a dict
card_network_tokenize_request_from_dict = CardNetworkTokenizeRequest.from_dict(card_network_tokenize_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


