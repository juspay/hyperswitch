# GooglePayAssuranceDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_holder_authenticated** | **bool** | indicates that Cardholder possession validation has been performed | 
**account_verified** | **bool** | indicates that identification and verifications (ID&amp;V) was performed | 

## Example

```python
from hyperswitch.models.google_pay_assurance_details import GooglePayAssuranceDetails

# TODO update the JSON string below
json = "{}"
# create an instance of GooglePayAssuranceDetails from a JSON string
google_pay_assurance_details_instance = GooglePayAssuranceDetails.from_json(json)
# print the JSON string representation of the object
print(GooglePayAssuranceDetails.to_json())

# convert the object into a dict
google_pay_assurance_details_dict = google_pay_assurance_details_instance.to_dict()
# create an instance of GooglePayAssuranceDetails from a dict
google_pay_assurance_details_from_dict = GooglePayAssuranceDetails.from_dict(google_pay_assurance_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


