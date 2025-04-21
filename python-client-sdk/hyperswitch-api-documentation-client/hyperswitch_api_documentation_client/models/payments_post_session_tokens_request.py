from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.payment_method import PaymentMethod
from ..models.payment_method_type import PaymentMethodType

T = TypeVar("T", bound="PaymentsPostSessionTokensRequest")


@_attrs_define
class PaymentsPostSessionTokensRequest:
    """
    Attributes:
        client_secret (str): It's a token used for client side verification.
        payment_method_type (PaymentMethodType): Indicates the sub type of payment method. Eg: 'google_pay' &
            'apple_pay' for wallets.
        payment_method (PaymentMethod): Indicates the type of payment method. Eg: 'card', 'wallet', etc.
    """

    client_secret: str
    payment_method_type: PaymentMethodType
    payment_method: PaymentMethod
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        client_secret = self.client_secret

        payment_method_type = self.payment_method_type.value

        payment_method = self.payment_method.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "client_secret": client_secret,
                "payment_method_type": payment_method_type,
                "payment_method": payment_method,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        client_secret = d.pop("client_secret")

        payment_method_type = PaymentMethodType(d.pop("payment_method_type"))

        payment_method = PaymentMethod(d.pop("payment_method"))

        payments_post_session_tokens_request = cls(
            client_secret=client_secret,
            payment_method_type=payment_method_type,
            payment_method=payment_method,
        )

        payments_post_session_tokens_request.additional_properties = d
        return payments_post_session_tokens_request

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
