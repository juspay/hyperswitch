# CardNetworkTokenizeResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_method_response** | [**PaymentMethodResponse**](PaymentMethodResponse.md) |  | [optional] 
**customer** | [**CustomerDetails**](CustomerDetails.md) |  | 
**card_tokenized** | **bool** | Card network tokenization status | 
**error_code** | **str** | Error code | [optional] 
**error_message** | **str** | Error message | [optional] 
**tokenization_data** | [**TokenizeDataRequest**](TokenizeDataRequest.md) |  | [optional] 

## Example

```python
from hyperswitch.models.card_network_tokenize_response import CardNetworkTokenizeResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CardNetworkTokenizeResponse from a JSON string
card_network_tokenize_response_instance = CardNetworkTokenizeResponse.from_json(json)
# print the JSON string representation of the object
print(CardNetworkTokenizeResponse.to_json())

# convert the object into a dict
card_network_tokenize_response_dict = card_network_tokenize_response_instance.to_dict()
# create an instance of CardNetworkTokenizeResponse from a dict
card_network_tokenize_response_from_dict = CardNetworkTokenizeResponse.from_dict(card_network_tokenize_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


