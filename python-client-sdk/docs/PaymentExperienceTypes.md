# PaymentExperienceTypes


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_experience_type** | [**PaymentExperience**](PaymentExperience.md) |  | [optional] 
**eligible_connectors** | **List[str]** | The list of eligible connectors for a given payment experience | 

## Example

```python
from hyperswitch.models.payment_experience_types import PaymentExperienceTypes

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentExperienceTypes from a JSON string
payment_experience_types_instance = PaymentExperienceTypes.from_json(json)
# print the JSON string representation of the object
print(PaymentExperienceTypes.to_json())

# convert the object into a dict
payment_experience_types_dict = payment_experience_types_instance.to_dict()
# create an instance of PaymentExperienceTypes from a dict
payment_experience_types_from_dict = PaymentExperienceTypes.from_dict(payment_experience_types_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


