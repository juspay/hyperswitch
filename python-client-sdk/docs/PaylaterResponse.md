# PaylaterResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**klarna_sdk** | [**KlarnaSdkPaymentMethodResponse**](KlarnaSdkPaymentMethodResponse.md) |  | [optional] 

## Example

```python
from hyperswitch.models.paylater_response import PaylaterResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaylaterResponse from a JSON string
paylater_response_instance = PaylaterResponse.from_json(json)
# print the JSON string representation of the object
print(PaylaterResponse.to_json())

# convert the object into a dict
paylater_response_dict = paylater_response_instance.to_dict()
# create an instance of PaylaterResponse from a dict
paylater_response_from_dict = PaylaterResponse.from_dict(paylater_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


