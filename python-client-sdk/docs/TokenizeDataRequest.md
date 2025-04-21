# TokenizeDataRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**TokenizeCardRequest**](TokenizeCardRequest.md) |  | 
**existing_payment_method** | [**TokenizePaymentMethodRequest**](TokenizePaymentMethodRequest.md) |  | 

## Example

```python
from hyperswitch.models.tokenize_data_request import TokenizeDataRequest

# TODO update the JSON string below
json = "{}"
# create an instance of TokenizeDataRequest from a JSON string
tokenize_data_request_instance = TokenizeDataRequest.from_json(json)
# print the JSON string representation of the object
print(TokenizeDataRequest.to_json())

# convert the object into a dict
tokenize_data_request_dict = tokenize_data_request_instance.to_dict()
# create an instance of TokenizeDataRequest from a dict
tokenize_data_request_from_dict = TokenizeDataRequest.from_dict(tokenize_data_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


