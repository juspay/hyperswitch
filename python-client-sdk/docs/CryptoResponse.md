# CryptoResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**pay_currency** | **str** |  | [optional] 
**network** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.crypto_response import CryptoResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CryptoResponse from a JSON string
crypto_response_instance = CryptoResponse.from_json(json)
# print the JSON string representation of the object
print(CryptoResponse.to_json())

# convert the object into a dict
crypto_response_dict = crypto_response_instance.to_dict()
# create an instance of CryptoResponse from a dict
crypto_response_from_dict = CryptoResponse.from_dict(crypto_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


