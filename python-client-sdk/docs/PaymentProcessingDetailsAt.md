# PaymentProcessingDetailsAt


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_processing_certificate** | **str** |  | 
**payment_processing_certificate_key** | **str** |  | 
**payment_processing_details_at** | **str** |  | 

## Example

```python
from hyperswitch.models.payment_processing_details_at import PaymentProcessingDetailsAt

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentProcessingDetailsAt from a JSON string
payment_processing_details_at_instance = PaymentProcessingDetailsAt.from_json(json)
# print the JSON string representation of the object
print(PaymentProcessingDetailsAt.to_json())

# convert the object into a dict
payment_processing_details_at_dict = payment_processing_details_at_instance.to_dict()
# create an instance of PaymentProcessingDetailsAt from a dict
payment_processing_details_at_from_dict = PaymentProcessingDetailsAt.from_dict(payment_processing_details_at_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


