from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.payment_method_type import PaymentMethodType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap


T = TypeVar("T", bound="PaymentsSessionRequest")


@_attrs_define
class PaymentsSessionRequest:
    """
    Attributes:
        payment_id (str): The identifier for the payment
        client_secret (str): This is a token which expires after 15 minutes, used from the client to authenticate and
            create sessions from the SDK
        wallets (list[PaymentMethodType]): The list of the supported wallets
        merchant_connector_details (Union['MerchantConnectorDetailsWrap', None, Unset]):
    """

    payment_id: str
    client_secret: str
    wallets: list[PaymentMethodType]
    merchant_connector_details: Union["MerchantConnectorDetailsWrap", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap

        payment_id = self.payment_id

        client_secret = self.client_secret

        wallets = []
        for wallets_item_data in self.wallets:
            wallets_item = wallets_item_data.value
            wallets.append(wallets_item)

        merchant_connector_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.merchant_connector_details, Unset):
            merchant_connector_details = UNSET
        elif isinstance(self.merchant_connector_details, MerchantConnectorDetailsWrap):
            merchant_connector_details = self.merchant_connector_details.to_dict()
        else:
            merchant_connector_details = self.merchant_connector_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_id": payment_id,
                "client_secret": client_secret,
                "wallets": wallets,
            }
        )
        if merchant_connector_details is not UNSET:
            field_dict["merchant_connector_details"] = merchant_connector_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap

        d = dict(src_dict)
        payment_id = d.pop("payment_id")

        client_secret = d.pop("client_secret")

        wallets = []
        _wallets = d.pop("wallets")
        for wallets_item_data in _wallets:
            wallets_item = PaymentMethodType(wallets_item_data)

            wallets.append(wallets_item)

        def _parse_merchant_connector_details(data: object) -> Union["MerchantConnectorDetailsWrap", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                merchant_connector_details_type_1 = MerchantConnectorDetailsWrap.from_dict(data)

                return merchant_connector_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorDetailsWrap", None, Unset], data)

        merchant_connector_details = _parse_merchant_connector_details(d.pop("merchant_connector_details", UNSET))

        payments_session_request = cls(
            payment_id=payment_id,
            client_secret=client_secret,
            wallets=wallets,
            merchant_connector_details=merchant_connector_details,
        )

        payments_session_request.additional_properties = d
        return payments_session_request

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
