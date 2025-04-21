# ProcessorPaymentToken

Processor payment token for MIT payments where payment_method_data is not available

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**processor_payment_token** | **str** |  | 
**merchant_connector_id** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.processor_payment_token import ProcessorPaymentToken

# TODO update the JSON string below
json = "{}"
# create an instance of ProcessorPaymentToken from a JSON string
processor_payment_token_instance = ProcessorPaymentToken.from_json(json)
# print the JSON string representation of the object
print(ProcessorPaymentToken.to_json())

# convert the object into a dict
processor_payment_token_dict = processor_payment_token_instance.to_dict()
# create an instance of ProcessorPaymentToken from a dict
processor_payment_token_from_dict = ProcessorPaymentToken.from_dict(processor_payment_token_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


