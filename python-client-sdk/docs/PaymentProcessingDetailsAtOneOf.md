# PaymentProcessingDetailsAtOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_processing_certificate** | **str** |  | 
**payment_processing_certificate_key** | **str** |  | 
**payment_processing_details_at** | **str** |  | 

## Example

```python
from hyperswitch.models.payment_processing_details_at_one_of import PaymentProcessingDetailsAtOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentProcessingDetailsAtOneOf from a JSON string
payment_processing_details_at_one_of_instance = PaymentProcessingDetailsAtOneOf.from_json(json)
# print the JSON string representation of the object
print(PaymentProcessingDetailsAtOneOf.to_json())

# convert the object into a dict
payment_processing_details_at_one_of_dict = payment_processing_details_at_one_of_instance.to_dict()
# create an instance of PaymentProcessingDetailsAtOneOf from a dict
payment_processing_details_at_one_of_from_dict = PaymentProcessingDetailsAtOneOf.from_dict(payment_processing_details_at_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


