from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.connector_status import ConnectorStatus
from ..models.connector_type import ConnectorType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.additional_merchant_data_type_0 import AdditionalMerchantDataType0
    from ..models.connector_wallet_details import ConnectorWalletDetails
    from ..models.frm_configs import FrmConfigs
    from ..models.merchant_connector_details import MerchantConnectorDetails
    from ..models.merchant_connector_update_metadata_type_0 import MerchantConnectorUpdateMetadataType0
    from ..models.merchant_connector_update_pm_auth_config_type_0 import MerchantConnectorUpdatePmAuthConfigType0
    from ..models.merchant_connector_webhook_details import MerchantConnectorWebhookDetails
    from ..models.payment_methods_enabled import PaymentMethodsEnabled


T = TypeVar("T", bound="MerchantConnectorUpdate")


@_attrs_define
class MerchantConnectorUpdate:
    """Create a new Merchant Connector for the merchant account. The connector could be a payment processor / facilitator /
    acquirer or specialized services like Fraud / Accounting etc."

        Attributes:
            connector_type (ConnectorType): Type of the Connector for the financial use case. Could range from Payments to
                Accounting to Banking.
            status (ConnectorStatus):
            connector_label (Union[None, Unset, str]): This is an unique label you can generate and pass in order to
                identify this connector account on your Hyperswitch dashboard and reports. Eg: if your profile label is
                `default`, connector label can be `stripe_default` Example: stripe_US_travel.
            connector_account_details (Union['MerchantConnectorDetails', None, Unset]):
            payment_methods_enabled (Union[None, Unset, list['PaymentMethodsEnabled']]): An object containing the details
                about the payment methods that need to be enabled under this merchant connector account Example:
                [{'accepted_countries': {'list': ['FR', 'DE', 'IN'], 'type': 'disable_only'}, 'accepted_currencies': {'list':
                ['USD', 'EUR'], 'type': 'enable_only'}, 'installment_payment_enabled': True, 'maximum_amount': 68607706,
                'minimum_amount': 1, 'payment_method': 'wallet', 'payment_method_issuers': ['labore magna ipsum', 'aute'],
                'payment_method_types': ['upi_collect', 'upi_intent'], 'payment_schemes': ['Discover', 'Discover'],
                'recurring_enabled': True}].
            connector_webhook_details (Union['MerchantConnectorWebhookDetails', None, Unset]):
            metadata (Union['MerchantConnectorUpdateMetadataType0', None, Unset]): Metadata is useful for storing
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
            pm_auth_config (Union['MerchantConnectorUpdatePmAuthConfigType0', None, Unset]): pm_auth_config will relate MCA
                records to their respective chosen auth services, based on payment_method and pmt
            additional_merchant_data (Union['AdditionalMerchantDataType0', None, Unset]):
            connector_wallets_details (Union['ConnectorWalletDetails', None, Unset]):
    """

    connector_type: ConnectorType
    status: ConnectorStatus
    connector_label: Union[None, Unset, str] = UNSET
    connector_account_details: Union["MerchantConnectorDetails", None, Unset] = UNSET
    payment_methods_enabled: Union[None, Unset, list["PaymentMethodsEnabled"]] = UNSET
    connector_webhook_details: Union["MerchantConnectorWebhookDetails", None, Unset] = UNSET
    metadata: Union["MerchantConnectorUpdateMetadataType0", None, Unset] = UNSET
    test_mode: Union[None, Unset, bool] = False
    disabled: Union[None, Unset, bool] = False
    frm_configs: Union[None, Unset, list["FrmConfigs"]] = UNSET
    pm_auth_config: Union["MerchantConnectorUpdatePmAuthConfigType0", None, Unset] = UNSET
    additional_merchant_data: Union["AdditionalMerchantDataType0", None, Unset] = UNSET
    connector_wallets_details: Union["ConnectorWalletDetails", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.additional_merchant_data_type_0 import AdditionalMerchantDataType0
        from ..models.connector_wallet_details import ConnectorWalletDetails
        from ..models.merchant_connector_details import MerchantConnectorDetails
        from ..models.merchant_connector_update_metadata_type_0 import MerchantConnectorUpdateMetadataType0
        from ..models.merchant_connector_update_pm_auth_config_type_0 import MerchantConnectorUpdatePmAuthConfigType0
        from ..models.merchant_connector_webhook_details import MerchantConnectorWebhookDetails

        connector_type = self.connector_type.value

        status = self.status.value

        connector_label: Union[None, Unset, str]
        if isinstance(self.connector_label, Unset):
            connector_label = UNSET
        else:
            connector_label = self.connector_label

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
        elif isinstance(self.metadata, MerchantConnectorUpdateMetadataType0):
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

        pm_auth_config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.pm_auth_config, Unset):
            pm_auth_config = UNSET
        elif isinstance(self.pm_auth_config, MerchantConnectorUpdatePmAuthConfigType0):
            pm_auth_config = self.pm_auth_config.to_dict()
        else:
            pm_auth_config = self.pm_auth_config

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
                "status": status,
            }
        )
        if connector_label is not UNSET:
            field_dict["connector_label"] = connector_label
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
        if pm_auth_config is not UNSET:
            field_dict["pm_auth_config"] = pm_auth_config
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
        from ..models.merchant_connector_details import MerchantConnectorDetails
        from ..models.merchant_connector_update_metadata_type_0 import MerchantConnectorUpdateMetadataType0
        from ..models.merchant_connector_update_pm_auth_config_type_0 import MerchantConnectorUpdatePmAuthConfigType0
        from ..models.merchant_connector_webhook_details import MerchantConnectorWebhookDetails
        from ..models.payment_methods_enabled import PaymentMethodsEnabled

        d = dict(src_dict)
        connector_type = ConnectorType(d.pop("connector_type"))

        status = ConnectorStatus(d.pop("status"))

        def _parse_connector_label(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_label = _parse_connector_label(d.pop("connector_label", UNSET))

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

        def _parse_metadata(data: object) -> Union["MerchantConnectorUpdateMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = MerchantConnectorUpdateMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorUpdateMetadataType0", None, Unset], data)

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

        def _parse_pm_auth_config(data: object) -> Union["MerchantConnectorUpdatePmAuthConfigType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                pm_auth_config_type_0 = MerchantConnectorUpdatePmAuthConfigType0.from_dict(data)

                return pm_auth_config_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorUpdatePmAuthConfigType0", None, Unset], data)

        pm_auth_config = _parse_pm_auth_config(d.pop("pm_auth_config", UNSET))

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

        merchant_connector_update = cls(
            connector_type=connector_type,
            status=status,
            connector_label=connector_label,
            connector_account_details=connector_account_details,
            payment_methods_enabled=payment_methods_enabled,
            connector_webhook_details=connector_webhook_details,
            metadata=metadata,
            test_mode=test_mode,
            disabled=disabled,
            frm_configs=frm_configs,
            pm_auth_config=pm_auth_config,
            additional_merchant_data=additional_merchant_data,
            connector_wallets_details=connector_wallets_details,
        )

        return merchant_connector_update
