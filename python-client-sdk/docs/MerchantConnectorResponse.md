# MerchantConnectorResponse

Response of creating a new Merchant Connector for the merchant account.\"

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector_type** | [**ConnectorType**](ConnectorType.md) |  | 
**connector_name** | [**Connector**](Connector.md) |  | 
**connector_label** | **str** | A unique label to identify the connector account created under a profile | [optional] 
**merchant_connector_id** | **str** | Unique ID of the merchant connector account | 
**profile_id** | **str** | Identifier for the profile, if not provided default will be chosen from merchant account | 
**connector_account_details** | [**MerchantConnectorDetails**](MerchantConnectorDetails.md) |  | [optional] 
**payment_methods_enabled** | [**List[PaymentMethodsEnabled]**](PaymentMethodsEnabled.md) | An object containing the details about the payment methods that need to be enabled under this merchant connector account | [optional] 
**connector_webhook_details** | [**MerchantConnectorWebhookDetails**](MerchantConnectorWebhookDetails.md) |  | [optional] 
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 
**test_mode** | **bool** | A boolean value to indicate if the connector is in Test mode. By default, its value is false. | [optional] [default to False]
**disabled** | **bool** | A boolean value to indicate if the connector is disabled. By default, its value is false. | [optional] [default to False]
**frm_configs** | [**List[FrmConfigs]**](FrmConfigs.md) | Contains the frm configs for the merchant connector | [optional] 
**business_country** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**business_label** | **str** | The business label to which the connector account is attached. To be deprecated soon. Use the &#39;profile_id&#39; instead | [optional] 
**business_sub_label** | **str** | The business sublabel to which the connector account is attached. To be deprecated soon. Use the &#39;profile_id&#39; instead | [optional] 
**applepay_verified_domains** | **List[str]** | identifier for the verified domains of a particular connector account | [optional] 
**pm_auth_config** | **object** |  | [optional] 
**status** | [**ConnectorStatus**](ConnectorStatus.md) |  | 
**additional_merchant_data** | [**AdditionalMerchantData**](AdditionalMerchantData.md) |  | [optional] 
**connector_wallets_details** | [**ConnectorWalletDetails**](ConnectorWalletDetails.md) |  | [optional] 

## Example

```python
from hyperswitch.models.merchant_connector_response import MerchantConnectorResponse

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantConnectorResponse from a JSON string
merchant_connector_response_instance = MerchantConnectorResponse.from_json(json)
# print the JSON string representation of the object
print(MerchantConnectorResponse.to_json())

# convert the object into a dict
merchant_connector_response_dict = merchant_connector_response_instance.to_dict()
# create an instance of MerchantConnectorResponse from a dict
merchant_connector_response_from_dict = MerchantConnectorResponse.from_dict(merchant_connector_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


