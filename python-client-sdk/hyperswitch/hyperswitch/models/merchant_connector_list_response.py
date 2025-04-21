from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.connector import Connector
from ..models.connector_status import ConnectorStatus
from ..models.connector_type import ConnectorType
from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.frm_configs import FrmConfigs
    from ..models.merchant_connector_list_response_pm_auth_config_type_0 import (
        MerchantConnectorListResponsePmAuthConfigType0,
    )
    from ..models.payment_methods_enabled import PaymentMethodsEnabled


T = TypeVar("T", bound="MerchantConnectorListResponse")


@_attrs_define
class MerchantConnectorListResponse:
    """
    Attributes:
        connector_type (ConnectorType): Type of the Connector for the financial use case. Could range from Payments to
            Accounting to Banking.
        connector_name (Connector):
        merchant_connector_id (str): Unique ID of the merchant connector account Example: mca_5apGeP94tMts6rg3U3kR.
        profile_id (str): Identifier for the profile, if not provided default will be chosen from merchant account
        status (ConnectorStatus):
        connector_label (Union[None, Unset, str]): A unique label to identify the connector account created under a
            profile Example: stripe_US_travel.
        payment_methods_enabled (Union[None, Unset, list['PaymentMethodsEnabled']]): An object containing the details
            about the payment methods that need to be enabled under this merchant connector account Example:
            [{'accepted_countries': {'list': ['FR', 'DE', 'IN'], 'type': 'disable_only'}, 'accepted_currencies': {'list':
            ['USD', 'EUR'], 'type': 'enable_only'}, 'installment_payment_enabled': True, 'maximum_amount': 68607706,
            'minimum_amount': 1, 'payment_method': 'wallet', 'payment_method_issuers': ['labore magna ipsum', 'aute'],
            'payment_method_types': ['upi_collect', 'upi_intent'], 'payment_schemes': ['Discover', 'Discover'],
            'recurring_enabled': True}].
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
            deprecated soon. Use the 'profile_id' instead Example: travel.
        business_sub_label (Union[None, Unset, str]): The business sublabel to which the connector account is attached.
            To be deprecated soon. Use the 'profile_id' instead Example: chase.
        applepay_verified_domains (Union[None, Unset, list[str]]): identifier for the verified domains of a particular
            connector account
        pm_auth_config (Union['MerchantConnectorListResponsePmAuthConfigType0', None, Unset]):
    """

    connector_type: ConnectorType
    connector_name: Connector
    merchant_connector_id: str
    profile_id: str
    status: ConnectorStatus
    connector_label: Union[None, Unset, str] = UNSET
    payment_methods_enabled: Union[None, Unset, list["PaymentMethodsEnabled"]] = UNSET
    test_mode: Union[None, Unset, bool] = False
    disabled: Union[None, Unset, bool] = False
    frm_configs: Union[None, Unset, list["FrmConfigs"]] = UNSET
    business_country: Union[CountryAlpha2, None, Unset] = UNSET
    business_label: Union[None, Unset, str] = UNSET
    business_sub_label: Union[None, Unset, str] = UNSET
    applepay_verified_domains: Union[None, Unset, list[str]] = UNSET
    pm_auth_config: Union["MerchantConnectorListResponsePmAuthConfigType0", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_connector_list_response_pm_auth_config_type_0 import (
            MerchantConnectorListResponsePmAuthConfigType0,
        )

        connector_type = self.connector_type.value

        connector_name = self.connector_name.value

        merchant_connector_id = self.merchant_connector_id

        profile_id = self.profile_id

        status = self.status.value

        connector_label: Union[None, Unset, str]
        if isinstance(self.connector_label, Unset):
            connector_label = UNSET
        else:
            connector_label = self.connector_label

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

        applepay_verified_domains: Union[None, Unset, list[str]]
        if isinstance(self.applepay_verified_domains, Unset):
            applepay_verified_domains = UNSET
        elif isinstance(self.applepay_verified_domains, list):
            applepay_verified_domains = self.applepay_verified_domains

        else:
            applepay_verified_domains = self.applepay_verified_domains

        pm_auth_config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.pm_auth_config, Unset):
            pm_auth_config = UNSET
        elif isinstance(self.pm_auth_config, MerchantConnectorListResponsePmAuthConfigType0):
            pm_auth_config = self.pm_auth_config.to_dict()
        else:
            pm_auth_config = self.pm_auth_config

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "connector_type": connector_type,
                "connector_name": connector_name,
                "merchant_connector_id": merchant_connector_id,
                "profile_id": profile_id,
                "status": status,
            }
        )
        if connector_label is not UNSET:
            field_dict["connector_label"] = connector_label
        if payment_methods_enabled is not UNSET:
            field_dict["payment_methods_enabled"] = payment_methods_enabled
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
        if applepay_verified_domains is not UNSET:
            field_dict["applepay_verified_domains"] = applepay_verified_domains
        if pm_auth_config is not UNSET:
            field_dict["pm_auth_config"] = pm_auth_config

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.frm_configs import FrmConfigs
        from ..models.merchant_connector_list_response_pm_auth_config_type_0 import (
            MerchantConnectorListResponsePmAuthConfigType0,
        )
        from ..models.payment_methods_enabled import PaymentMethodsEnabled

        d = dict(src_dict)
        connector_type = ConnectorType(d.pop("connector_type"))

        connector_name = Connector(d.pop("connector_name"))

        merchant_connector_id = d.pop("merchant_connector_id")

        profile_id = d.pop("profile_id")

        status = ConnectorStatus(d.pop("status"))

        def _parse_connector_label(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_label = _parse_connector_label(d.pop("connector_label", UNSET))

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

        def _parse_applepay_verified_domains(data: object) -> Union[None, Unset, list[str]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                applepay_verified_domains_type_0 = cast(list[str], data)

                return applepay_verified_domains_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[str]], data)

        applepay_verified_domains = _parse_applepay_verified_domains(d.pop("applepay_verified_domains", UNSET))

        def _parse_pm_auth_config(data: object) -> Union["MerchantConnectorListResponsePmAuthConfigType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                pm_auth_config_type_0 = MerchantConnectorListResponsePmAuthConfigType0.from_dict(data)

                return pm_auth_config_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorListResponsePmAuthConfigType0", None, Unset], data)

        pm_auth_config = _parse_pm_auth_config(d.pop("pm_auth_config", UNSET))

        merchant_connector_list_response = cls(
            connector_type=connector_type,
            connector_name=connector_name,
            merchant_connector_id=merchant_connector_id,
            profile_id=profile_id,
            status=status,
            connector_label=connector_label,
            payment_methods_enabled=payment_methods_enabled,
            test_mode=test_mode,
            disabled=disabled,
            frm_configs=frm_configs,
            business_country=business_country,
            business_label=business_label,
            business_sub_label=business_sub_label,
            applepay_verified_domains=applepay_verified_domains,
            pm_auth_config=pm_auth_config,
        )

        return merchant_connector_list_response
