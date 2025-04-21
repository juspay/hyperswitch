# EphemeralKeyCreateResponse

ephemeral_key for the customer_id mentioned

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**customer_id** | **str** | customer_id to which this ephemeral key belongs to | 
**created_at** | **int** | time at which this ephemeral key was created | 
**expires** | **int** | time at which this ephemeral key would expire | 
**secret** | **str** | ephemeral key | 

## Example

```python
from hyperswitch.models.ephemeral_key_create_response import EphemeralKeyCreateResponse

# TODO update the JSON string below
json = "{}"
# create an instance of EphemeralKeyCreateResponse from a JSON string
ephemeral_key_create_response_instance = EphemeralKeyCreateResponse.from_json(json)
# print the JSON string representation of the object
print(EphemeralKeyCreateResponse.to_json())

# convert the object into a dict
ephemeral_key_create_response_dict = ephemeral_key_create_response_instance.to_dict()
# create an instance of EphemeralKeyCreateResponse from a dict
ephemeral_key_create_response_from_dict = EphemeralKeyCreateResponse.from_dict(ephemeral_key_create_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


