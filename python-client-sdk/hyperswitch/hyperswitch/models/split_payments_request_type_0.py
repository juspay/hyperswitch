from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.stripe_split_payment_request import StripeSplitPaymentRequest


T = TypeVar("T", bound="SplitPaymentsRequestType0")


@_attrs_define
class SplitPaymentsRequestType0:
    """
    Attributes:
        stripe_split_payment (StripeSplitPaymentRequest): Fee information for Split Payments to be charged on the
            payment being collected for Stripe
    """

    stripe_split_payment: "StripeSplitPaymentRequest"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        stripe_split_payment = self.stripe_split_payment.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "stripe_split_payment": stripe_split_payment,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.stripe_split_payment_request import StripeSplitPaymentRequest

        d = dict(src_dict)
        stripe_split_payment = StripeSplitPaymentRequest.from_dict(d.pop("stripe_split_payment"))

        split_payments_request_type_0 = cls(
            stripe_split_payment=stripe_split_payment,
        )

        split_payments_request_type_0.additional_properties = d
        return split_payments_request_type_0

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
