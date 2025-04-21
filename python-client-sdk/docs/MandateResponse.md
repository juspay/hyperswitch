# MandateResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**mandate_id** | **str** | The identifier for mandate | 
**status** | [**MandateStatus**](MandateStatus.md) |  | 
**payment_method_id** | **str** | The identifier for payment method | 
**payment_method** | **str** | The payment method | 
**payment_method_type** | **str** | The payment method type | [optional] 
**card** | [**MandateCardDetails**](MandateCardDetails.md) |  | [optional] 
**customer_acceptance** | [**CustomerAcceptance**](CustomerAcceptance.md) |  | [optional] 

## Example

```python
from hyperswitch.models.mandate_response import MandateResponse

# TODO update the JSON string below
json = "{}"
# create an instance of MandateResponse from a JSON string
mandate_response_instance = MandateResponse.from_json(json)
# print the JSON string representation of the object
print(MandateResponse.to_json())

# convert the object into a dict
mandate_response_dict = mandate_response_instance.to_dict()
# create an instance of MandateResponse from a dict
mandate_response_from_dict = MandateResponse.from_dict(mandate_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


