# TokenizeDataRequestOneOf1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**existing_payment_method** | [**TokenizePaymentMethodRequest**](TokenizePaymentMethodRequest.md) |  | 

## Example

```python
from hyperswitch.models.tokenize_data_request_one_of1 import TokenizeDataRequestOneOf1

# TODO update the JSON string below
json = "{}"
# create an instance of TokenizeDataRequestOneOf1 from a JSON string
tokenize_data_request_one_of1_instance = TokenizeDataRequestOneOf1.from_json(json)
# print the JSON string representation of the object
print(TokenizeDataRequestOneOf1.to_json())

# convert the object into a dict
tokenize_data_request_one_of1_dict = tokenize_data_request_one_of1_instance.to_dict()
# create an instance of TokenizeDataRequestOneOf1 from a dict
tokenize_data_request_one_of1_from_dict = TokenizeDataRequestOneOf1.from_dict(tokenize_data_request_one_of1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


