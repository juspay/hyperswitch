from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.gpay_allowed_payment_methods import GpayAllowedPaymentMethods
    from ..models.gpay_merchant_info import GpayMerchantInfo
    from ..models.gpay_shipping_address_parameters import GpayShippingAddressParameters
    from ..models.gpay_transaction_info import GpayTransactionInfo
    from ..models.sdk_next_action import SdkNextAction
    from ..models.secret_info_to_initiate_sdk import SecretInfoToInitiateSdk


T = TypeVar("T", bound="GooglePaySessionResponse")


@_attrs_define
class GooglePaySessionResponse:
    """
    Attributes:
        merchant_info (GpayMerchantInfo):
        shipping_address_required (bool): Is shipping address required
        email_required (bool): Is email required
        shipping_address_parameters (GpayShippingAddressParameters):
        allowed_payment_methods (list['GpayAllowedPaymentMethods']): List of the allowed payment meythods
        transaction_info (GpayTransactionInfo):
        delayed_session_token (bool): Identifier for the delayed session response
        connector (str): The name of the connector
        sdk_next_action (SdkNextAction):
        secrets (Union['SecretInfoToInitiateSdk', None, Unset]):
    """

    merchant_info: "GpayMerchantInfo"
    shipping_address_required: bool
    email_required: bool
    shipping_address_parameters: "GpayShippingAddressParameters"
    allowed_payment_methods: list["GpayAllowedPaymentMethods"]
    transaction_info: "GpayTransactionInfo"
    delayed_session_token: bool
    connector: str
    sdk_next_action: "SdkNextAction"
    secrets: Union["SecretInfoToInitiateSdk", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.secret_info_to_initiate_sdk import SecretInfoToInitiateSdk

        merchant_info = self.merchant_info.to_dict()

        shipping_address_required = self.shipping_address_required

        email_required = self.email_required

        shipping_address_parameters = self.shipping_address_parameters.to_dict()

        allowed_payment_methods = []
        for allowed_payment_methods_item_data in self.allowed_payment_methods:
            allowed_payment_methods_item = allowed_payment_methods_item_data.to_dict()
            allowed_payment_methods.append(allowed_payment_methods_item)

        transaction_info = self.transaction_info.to_dict()

        delayed_session_token = self.delayed_session_token

        connector = self.connector

        sdk_next_action = self.sdk_next_action.to_dict()

        secrets: Union[None, Unset, dict[str, Any]]
        if isinstance(self.secrets, Unset):
            secrets = UNSET
        elif isinstance(self.secrets, SecretInfoToInitiateSdk):
            secrets = self.secrets.to_dict()
        else:
            secrets = self.secrets

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "merchant_info": merchant_info,
                "shipping_address_required": shipping_address_required,
                "email_required": email_required,
                "shipping_address_parameters": shipping_address_parameters,
                "allowed_payment_methods": allowed_payment_methods,
                "transaction_info": transaction_info,
                "delayed_session_token": delayed_session_token,
                "connector": connector,
                "sdk_next_action": sdk_next_action,
            }
        )
        if secrets is not UNSET:
            field_dict["secrets"] = secrets

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.gpay_allowed_payment_methods import GpayAllowedPaymentMethods
        from ..models.gpay_merchant_info import GpayMerchantInfo
        from ..models.gpay_shipping_address_parameters import GpayShippingAddressParameters
        from ..models.gpay_transaction_info import GpayTransactionInfo
        from ..models.sdk_next_action import SdkNextAction
        from ..models.secret_info_to_initiate_sdk import SecretInfoToInitiateSdk

        d = dict(src_dict)
        merchant_info = GpayMerchantInfo.from_dict(d.pop("merchant_info"))

        shipping_address_required = d.pop("shipping_address_required")

        email_required = d.pop("email_required")

        shipping_address_parameters = GpayShippingAddressParameters.from_dict(d.pop("shipping_address_parameters"))

        allowed_payment_methods = []
        _allowed_payment_methods = d.pop("allowed_payment_methods")
        for allowed_payment_methods_item_data in _allowed_payment_methods:
            allowed_payment_methods_item = GpayAllowedPaymentMethods.from_dict(allowed_payment_methods_item_data)

            allowed_payment_methods.append(allowed_payment_methods_item)

        transaction_info = GpayTransactionInfo.from_dict(d.pop("transaction_info"))

        delayed_session_token = d.pop("delayed_session_token")

        connector = d.pop("connector")

        sdk_next_action = SdkNextAction.from_dict(d.pop("sdk_next_action"))

        def _parse_secrets(data: object) -> Union["SecretInfoToInitiateSdk", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                secrets_type_1 = SecretInfoToInitiateSdk.from_dict(data)

                return secrets_type_1
            except:  # noqa: E722
                pass
            return cast(Union["SecretInfoToInitiateSdk", None, Unset], data)

        secrets = _parse_secrets(d.pop("secrets", UNSET))

        google_pay_session_response = cls(
            merchant_info=merchant_info,
            shipping_address_required=shipping_address_required,
            email_required=email_required,
            shipping_address_parameters=shipping_address_parameters,
            allowed_payment_methods=allowed_payment_methods,
            transaction_info=transaction_info,
            delayed_session_token=delayed_session_token,
            connector=connector,
            sdk_next_action=sdk_next_action,
            secrets=secrets,
        )

        google_pay_session_response.additional_properties = d
        return google_pay_session_response

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
