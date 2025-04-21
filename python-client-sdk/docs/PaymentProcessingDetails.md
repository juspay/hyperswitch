# PaymentProcessingDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_processing_certificate** | **str** |  | 
**payment_processing_certificate_key** | **str** |  | 

## Example

```python
from hyperswitch.models.payment_processing_details import PaymentProcessingDetails

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentProcessingDetails from a JSON string
payment_processing_details_instance = PaymentProcessingDetails.from_json(json)
# print the JSON string representation of the object
print(PaymentProcessingDetails.to_json())

# convert the object into a dict
payment_processing_details_dict = payment_processing_details_instance.to_dict()
# create an instance of PaymentProcessingDetails from a dict
payment_processing_details_from_dict = PaymentProcessingDetails.from_dict(payment_processing_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


