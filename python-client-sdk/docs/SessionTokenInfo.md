# SessionTokenInfo


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_processing_certificate** | **str** |  | 
**payment_processing_certificate_key** | **str** |  | 
**payment_processing_details_at** | **str** |  | 
**certificate** | **str** |  | 
**certificate_keys** | **str** |  | 
**merchant_identifier** | **str** |  | 
**display_name** | **str** |  | 
**initiative** | [**ApplepayInitiative**](ApplepayInitiative.md) |  | 
**initiative_context** | **str** |  | [optional] 
**merchant_business_country** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 

## Example

```python
from hyperswitch.models.session_token_info import SessionTokenInfo

# TODO update the JSON string below
json = "{}"
# create an instance of SessionTokenInfo from a JSON string
session_token_info_instance = SessionTokenInfo.from_json(json)
# print the JSON string representation of the object
print(SessionTokenInfo.to_json())

# convert the object into a dict
session_token_info_dict = session_token_info_instance.to_dict()
# create an instance of SessionTokenInfo from a dict
session_token_info_from_dict = SessionTokenInfo.from_dict(session_token_info_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


