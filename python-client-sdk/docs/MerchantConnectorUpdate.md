# MerchantConnectorUpdate

Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator / acquirer or specialized services like Fraud / Accounting etc.\"

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector_type** | [**ConnectorType**](ConnectorType.md) |  | 
**connector_label** | **str** | This is an unique label you can generate and pass in order to identify this connector account on your Hyperswitch dashboard and reports. Eg: if your profile label is &#x60;default&#x60;, connector label can be &#x60;stripe_default&#x60; | [optional] 
**connector_account_details** | [**MerchantConnectorDetails**](MerchantConnectorDetails.md) |  | [optional] 
**payment_methods_enabled** | [**List[PaymentMethodsEnabled]**](PaymentMethodsEnabled.md) | An object containing the details about the payment methods that need to be enabled under this merchant connector account | [optional] 
**connector_webhook_details** | [**MerchantConnectorWebhookDetails**](MerchantConnectorWebhookDetails.md) |  | [optional] 
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 
**test_mode** | **bool** | A boolean value to indicate if the connector is in Test mode. By default, its value is false. | [optional] [default to False]
**disabled** | **bool** | A boolean value to indicate if the connector is disabled. By default, its value is false. | [optional] [default to False]
**frm_configs** | [**List[FrmConfigs]**](FrmConfigs.md) | Contains the frm configs for the merchant connector | [optional] 
**pm_auth_config** | **object** | pm_auth_config will relate MCA records to their respective chosen auth services, based on payment_method and pmt | [optional] 
**status** | [**ConnectorStatus**](ConnectorStatus.md) |  | 
**additional_merchant_data** | [**AdditionalMerchantData**](AdditionalMerchantData.md) |  | [optional] 
**connector_wallets_details** | [**ConnectorWalletDetails**](ConnectorWalletDetails.md) |  | [optional] 

## Example

```python
from hyperswitch.models.merchant_connector_update import MerchantConnectorUpdate

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantConnectorUpdate from a JSON string
merchant_connector_update_instance = MerchantConnectorUpdate.from_json(json)
# print the JSON string representation of the object
print(MerchantConnectorUpdate.to_json())

# convert the object into a dict
merchant_connector_update_dict = merchant_connector_update_instance.to_dict()
# create an instance of MerchantConnectorUpdate from a dict
merchant_connector_update_from_dict = MerchantConnectorUpdate.from_dict(merchant_connector_update_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


