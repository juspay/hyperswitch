from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.connector import Connector
from ..models.connector_status import ConnectorStatus
from ..models.connector_type import ConnectorType
from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.additional_merchant_data_type_0 import AdditionalMerchantDataType0
    from ..models.connector_wallet_details import ConnectorWalletDetails
    from ..models.frm_configs import FrmConfigs
    from ..models.merchant_connector_create_metadata_type_0 import MerchantConnectorCreateMetadataType0
    from ..models.merchant_connector_create_pm_auth_config_type_0 import MerchantConnectorCreatePmAuthConfigType0
    from ..models.merchant_connector_details import MerchantConnectorDetails
    from ..models.merchant_connector_webhook_details import MerchantConnectorWebhookDetails
    from ..models.payment_methods_enabled import PaymentMethodsEnabled


T = TypeVar("T", bound="MerchantConnectorCreate")


@_attrs_define
class MerchantConnectorCreate:
    """Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator /
    acquirer or specialized services like Fraud / Accounting etc."

        Attributes:
            connector_type (ConnectorType): Type of the Connector for the financial use case. Could range from Payments to
                Accounting to Banking.
            connector_name (Connector):
            connector_label (Union[None, Unset, str]): This is an unique label you can generate and pass in order to
                identify this connector account on your Hyperswitch dashboard and reports. Eg: if your profile label is
                `default`, connector label can be `stripe_default` Example: stripe_US_travel.
            profile_id (Union[None, Unset, str]): Identifier for the profile, if not provided default will be chosen from
                merchant account
            connector_account_details (Union['MerchantConnectorDetails', None, Unset]):
            payment_methods_enabled (Union[None, Unset, list['PaymentMethodsEnabled']]): An object containing the details
                about the payment methods that need to be enabled under this merchant connector account Example:
                [{'accepted_countries': {'list': ['FR', 'DE', 'IN'], 'type': 'disable_only'}, 'accepted_currencies': {'list':
                ['USD', 'EUR'], 'type': 'enable_only'}, 'installment_payment_enabled': True, 'maximum_amount': 68607706,
                'minimum_amount': 1, 'payment_method': 'wallet', 'payment_method_issuers': ['labore magna ipsum', 'aute'],
                'payment_method_types': ['upi_collect', 'upi_intent'], 'payment_schemes': ['Discover', 'Discover'],
                'recurring_enabled': True}].
            connector_webhook_details (Union['MerchantConnectorWebhookDetails', None, Unset]):
            metadata (Union['MerchantConnectorCreateMetadataType0', None, Unset]): Metadata is useful for storing
                additional, unstructured information on an object.
            test_mode (Union[None, Unset, bool]): A boolean value to indicate if the connector is in Test mode. By default,
                its value is false. Default: False.
            disabled (Union[None, Unset, bool]): A boolean value to indicate if the connector is disabled. By default, its
                value is false. Default: False.
            frm_configs (Union[None, Unset, list['FrmConfigs']]): Contains the frm configs for the merchant connector
                Example:
                [{"gateway":"stripe","payment_methods":[{"payment_method":"card","payment_method_types":[{"payment_method_type":
                "credit","card_networks":["Visa"],"flow":"pre","action":"cancel_txn"},{"payment_method_type":"debit","card_netwo
                rks":["Visa"],"flow":"pre"}]}]}]
                .
            business_country (Union[CountryAlpha2, None, Unset]):
            business_label (Union[None, Unset, str]): The business label to which the connector account is attached. To be
                deprecated soon. Use the 'profile_id' instead
            business_sub_label (Union[None, Unset, str]): The business sublabel to which the connector account is attached.
                To be deprecated soon. Use the 'profile_id' instead Example: chase.
            merchant_connector_id (Union[None, Unset, str]): Unique ID of the connector Example: mca_5apGeP94tMts6rg3U3kR.
            pm_auth_config (Union['MerchantConnectorCreatePmAuthConfigType0', None, Unset]):
            status (Union[ConnectorStatus, None, Unset]):
            additional_merchant_data (Union['AdditionalMerchantDataType0', None, Unset]):
            connector_wallets_details (Union['ConnectorWalletDetails', None, Unset]):
    """

    connector_type: ConnectorType
    connector_name: Connector
    connector_label: Union[None, Unset, str] = UNSET
    profile_id: Union[None, Unset, str] = UNSET
    connector_account_details: Union["MerchantConnectorDetails", None, Unset] = UNSET
    payment_methods_enabled: Union[None, Unset, list["PaymentMethodsEnabled"]] = UNSET
    connector_webhook_details: Union["MerchantConnectorWebhookDetails", None, Unset] = UNSET
    metadata: Union["MerchantConnectorCreateMetadataType0", None, Unset] = UNSET
    test_mode: Union[None, Unset, bool] = False
    disabled: Union[None, Unset, bool] = False
    frm_configs: Union[None, Unset, list["FrmConfigs"]] = UNSET
    business_country: Union[CountryAlpha2, None, Unset] = UNSET
    business_label: Union[None, Unset, str] = UNSET
    business_sub_label: Union[None, Unset, str] = UNSET
    merchant_connector_id: Union[None, Unset, str] = UNSET
    pm_auth_config: Union["MerchantConnectorCreatePmAuthConfigType0", None, Unset] = UNSET
    status: Union[ConnectorStatus, None, Unset] = UNSET
    additional_merchant_data: Union["AdditionalMerchantDataType0", None, Unset] = UNSET
    connector_wallets_details: Union["ConnectorWalletDetails", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.additional_merchant_data_type_0 import AdditionalMerchantDataType0
        from ..models.connector_wallet_details import ConnectorWalletDetails
        from ..models.merchant_connector_create_metadata_type_0 import MerchantConnectorCreateMetadataType0
        from ..models.merchant_connector_create_pm_auth_config_type_0 import MerchantConnectorCreatePmAuthConfigType0
        from ..models.merchant_connector_details import MerchantConnectorDetails
        from ..models.merchant_connector_webhook_details import MerchantConnectorWebhookDetails

        connector_type = self.connector_type.value

        connector_name = self.connector_name.value

        connector_label: Union[None, Unset, str]
        if isinstance(self.connector_label, Unset):
            connector_label = UNSET
        else:
            connector_label = self.connector_label

        profile_id: Union[None, Unset, str]
        if isinstance(self.profile_id, Unset):
            profile_id = UNSET
        else:
            profile_id = self.profile_id

        connector_account_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.connector_account_details, Unset):
            connector_account_details = UNSET
        elif isinstance(self.connector_account_details, MerchantConnectorDetails):
            connector_account_details = self.connector_account_details.to_dict()
        else:
            connector_account_details = self.connector_account_details

        payment_methods_enabled: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.payment_methods_enabled, Unset):
            payment_methods_enabled = UNSET
        elif isinstance(self.payment_methods_enabled, list):
            payment_methods_enabled = []
            for payment_methods_enabled_type_0_item_data in self.payment_methods_enabled:
                payment_methods_enabled_type_0_item = payment_methods_enabled_type_0_item_data.to_dict()
                payment_methods_enabled.append(payment_methods_enabled_type_0_item)

        else:
            payment_methods_enabled = self.payment_methods_enabled

        connector_webhook_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.connector_webhook_details, Unset):
            connector_webhook_details = UNSET
        elif isinstance(self.connector_webhook_details, MerchantConnectorWebhookDetails):
            connector_webhook_details = self.connector_webhook_details.to_dict()
        else:
            connector_webhook_details = self.connector_webhook_details

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, MerchantConnectorCreateMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        test_mode: Union[None, Unset, bool]
        if isinstance(self.test_mode, Unset):
            test_mode = UNSET
        else:
            test_mode = self.test_mode

        disabled: Union[None, Unset, bool]
        if isinstance(self.disabled, Unset):
            disabled = UNSET
        else:
            disabled = self.disabled

        frm_configs: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.frm_configs, Unset):
            frm_configs = UNSET
        elif isinstance(self.frm_configs, list):
            frm_configs = []
            for frm_configs_type_0_item_data in self.frm_configs:
                frm_configs_type_0_item = frm_configs_type_0_item_data.to_dict()
                frm_configs.append(frm_configs_type_0_item)

        else:
            frm_configs = self.frm_configs

        business_country: Union[None, Unset, str]
        if isinstance(self.business_country, Unset):
            business_country = UNSET
        elif isinstance(self.business_country, CountryAlpha2):
            business_country = self.business_country.value
        else:
            business_country = self.business_country

        business_label: Union[None, Unset, str]
        if isinstance(self.business_label, Unset):
            business_label = UNSET
        else:
            business_label = self.business_label

        business_sub_label: Union[None, Unset, str]
        if isinstance(self.business_sub_label, Unset):
            business_sub_label = UNSET
        else:
            business_sub_label = self.business_sub_label

        merchant_connector_id: Union[None, Unset, str]
        if isinstance(self.merchant_connector_id, Unset):
            merchant_connector_id = UNSET
        else:
            merchant_connector_id = self.merchant_connector_id

        pm_auth_config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.pm_auth_config, Unset):
            pm_auth_config = UNSET
        elif isinstance(self.pm_auth_config, MerchantConnectorCreatePmAuthConfigType0):
            pm_auth_config = self.pm_auth_config.to_dict()
        else:
            pm_auth_config = self.pm_auth_config

        status: Union[None, Unset, str]
        if isinstance(self.status, Unset):
            status = UNSET
        elif isinstance(self.status, ConnectorStatus):
            status = self.status.value
        else:
            status = self.status

        additional_merchant_data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.additional_merchant_data, Unset):
            additional_merchant_data = UNSET
        elif isinstance(self.additional_merchant_data, AdditionalMerchantDataType0):
            additional_merchant_data = self.additional_merchant_data.to_dict()
        else:
            additional_merchant_data = self.additional_merchant_data

        connector_wallets_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.connector_wallets_details, Unset):
            connector_wallets_details = UNSET
        elif isinstance(self.connector_wallets_details, ConnectorWalletDetails):
            connector_wallets_details = self.connector_wallets_details.to_dict()
        else:
            connector_wallets_details = self.connector_wallets_details

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "connector_type": connector_type,
                "connector_name": connector_name,
            }
        )
        if connector_label is not UNSET:
            field_dict["connector_label"] = connector_label
        if profile_id is not UNSET:
            field_dict["profile_id"] = profile_id
        if connector_account_details is not UNSET:
            field_dict["connector_account_details"] = connector_account_details
        if payment_methods_enabled is not UNSET:
            field_dict["payment_methods_enabled"] = payment_methods_enabled
        if connector_webhook_details is not UNSET:
            field_dict["connector_webhook_details"] = connector_webhook_details
        if metadata is not UNSET:
            field_dict["metadata"] = metadata
        if test_mode is not UNSET:
            field_dict["test_mode"] = test_mode
        if disabled is not UNSET:
            field_dict["disabled"] = disabled
        if frm_configs is not UNSET:
            field_dict["frm_configs"] = frm_configs
        if business_country is not UNSET:
            field_dict["business_country"] = business_country
        if business_label is not UNSET:
            field_dict["business_label"] = business_label
        if business_sub_label is not UNSET:
            field_dict["business_sub_label"] = business_sub_label
        if merchant_connector_id is not UNSET:
            field_dict["merchant_connector_id"] = merchant_connector_id
        if pm_auth_config is not UNSET:
            field_dict["pm_auth_config"] = pm_auth_config
        if status is not UNSET:
            field_dict["status"] = status
        if additional_merchant_data is not UNSET:
            field_dict["additional_merchant_data"] = additional_merchant_data
        if connector_wallets_details is not UNSET:
            field_dict["connector_wallets_details"] = connector_wallets_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.additional_merchant_data_type_0 import AdditionalMerchantDataType0
        from ..models.connector_wallet_details import ConnectorWalletDetails
        from ..models.frm_configs import FrmConfigs
        from ..models.merchant_connector_create_metadata_type_0 import MerchantConnectorCreateMetadataType0
        from ..models.merchant_connector_create_pm_auth_config_type_0 import MerchantConnectorCreatePmAuthConfigType0
        from ..models.merchant_connector_details import MerchantConnectorDetails
        from ..models.merchant_connector_webhook_details import MerchantConnectorWebhookDetails
        from ..models.payment_methods_enabled import PaymentMethodsEnabled

        d = dict(src_dict)
        connector_type = ConnectorType(d.pop("connector_type"))

        connector_name = Connector(d.pop("connector_name"))

        def _parse_connector_label(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_label = _parse_connector_label(d.pop("connector_label", UNSET))

        def _parse_profile_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        profile_id = _parse_profile_id(d.pop("profile_id", UNSET))

        def _parse_connector_account_details(data: object) -> Union["MerchantConnectorDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                connector_account_details_type_1 = MerchantConnectorDetails.from_dict(data)

                return connector_account_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorDetails", None, Unset], data)

        connector_account_details = _parse_connector_account_details(d.pop("connector_account_details", UNSET))

        def _parse_payment_methods_enabled(data: object) -> Union[None, Unset, list["PaymentMethodsEnabled"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                payment_methods_enabled_type_0 = []
                _payment_methods_enabled_type_0 = data
                for payment_methods_enabled_type_0_item_data in _payment_methods_enabled_type_0:
                    payment_methods_enabled_type_0_item = PaymentMethodsEnabled.from_dict(
                        payment_methods_enabled_type_0_item_data
                    )

                    payment_methods_enabled_type_0.append(payment_methods_enabled_type_0_item)

                return payment_methods_enabled_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["PaymentMethodsEnabled"]], data)

        payment_methods_enabled = _parse_payment_methods_enabled(d.pop("payment_methods_enabled", UNSET))

        def _parse_connector_webhook_details(data: object) -> Union["MerchantConnectorWebhookDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                connector_webhook_details_type_1 = MerchantConnectorWebhookDetails.from_dict(data)

                return connector_webhook_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorWebhookDetails", None, Unset], data)

        connector_webhook_details = _parse_connector_webhook_details(d.pop("connector_webhook_details", UNSET))

        def _parse_metadata(data: object) -> Union["MerchantConnectorCreateMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = MerchantConnectorCreateMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorCreateMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        def _parse_test_mode(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        test_mode = _parse_test_mode(d.pop("test_mode", UNSET))

        def _parse_disabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        disabled = _parse_disabled(d.pop("disabled", UNSET))

        def _parse_frm_configs(data: object) -> Union[None, Unset, list["FrmConfigs"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                frm_configs_type_0 = []
                _frm_configs_type_0 = data
                for frm_configs_type_0_item_data in _frm_configs_type_0:
                    frm_configs_type_0_item = FrmConfigs.from_dict(frm_configs_type_0_item_data)

                    frm_configs_type_0.append(frm_configs_type_0_item)

                return frm_configs_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["FrmConfigs"]], data)

        frm_configs = _parse_frm_configs(d.pop("frm_configs", UNSET))

        def _parse_business_country(data: object) -> Union[CountryAlpha2, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                business_country_type_1 = CountryAlpha2(data)

                return business_country_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CountryAlpha2, None, Unset], data)

        business_country = _parse_business_country(d.pop("business_country", UNSET))

        def _parse_business_label(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        business_label = _parse_business_label(d.pop("business_label", UNSET))

        def _parse_business_sub_label(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        business_sub_label = _parse_business_sub_label(d.pop("business_sub_label", UNSET))

        def _parse_merchant_connector_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_connector_id = _parse_merchant_connector_id(d.pop("merchant_connector_id", UNSET))

        def _parse_pm_auth_config(data: object) -> Union["MerchantConnectorCreatePmAuthConfigType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                pm_auth_config_type_0 = MerchantConnectorCreatePmAuthConfigType0.from_dict(data)

                return pm_auth_config_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorCreatePmAuthConfigType0", None, Unset], data)

        pm_auth_config = _parse_pm_auth_config(d.pop("pm_auth_config", UNSET))

        def _parse_status(data: object) -> Union[ConnectorStatus, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                status_type_1 = ConnectorStatus(data)

                return status_type_1
            except:  # noqa: E722
                pass
            return cast(Union[ConnectorStatus, None, Unset], data)

        status = _parse_status(d.pop("status", UNSET))

        def _parse_additional_merchant_data(data: object) -> Union["AdditionalMerchantDataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_additional_merchant_data_type_0 = AdditionalMerchantDataType0.from_dict(data)

                return componentsschemas_additional_merchant_data_type_0
            except:  # noqa: E722
                pass
            return cast(Union["AdditionalMerchantDataType0", None, Unset], data)

        additional_merchant_data = _parse_additional_merchant_data(d.pop("additional_merchant_data", UNSET))

        def _parse_connector_wallets_details(data: object) -> Union["ConnectorWalletDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                connector_wallets_details_type_1 = ConnectorWalletDetails.from_dict(data)

                return connector_wallets_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["ConnectorWalletDetails", None, Unset], data)

        connector_wallets_details = _parse_connector_wallets_details(d.pop("connector_wallets_details", UNSET))

        merchant_connector_create = cls(
            connector_type=connector_type,
            connector_name=connector_name,
            connector_label=connector_label,
            profile_id=profile_id,
            connector_account_details=connector_account_details,
            payment_methods_enabled=payment_methods_enabled,
            connector_webhook_details=connector_webhook_details,
            metadata=metadata,
            test_mode=test_mode,
            disabled=disabled,
            frm_configs=frm_configs,
            business_country=business_country,
            business_label=business_label,
            business_sub_label=business_sub_label,
            merchant_connector_id=merchant_connector_id,
            pm_auth_config=pm_auth_config,
            status=status,
            additional_merchant_data=additional_merchant_data,
            connector_wallets_details=connector_wallets_details,
        )

        return merchant_connector_create
