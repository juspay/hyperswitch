# CryptoData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**pay_currency** | **str** |  | [optional] 
**network** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.crypto_data import CryptoData

# TODO update the JSON string below
json = "{}"
# create an instance of CryptoData from a JSON string
crypto_data_instance = CryptoData.from_json(json)
# print the JSON string representation of the object
print(CryptoData.to_json())

# convert the object into a dict
crypto_data_dict = crypto_data_instance.to_dict()
# create an instance of CryptoData from a dict
crypto_data_from_dict = CryptoData.from_dict(crypto_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


