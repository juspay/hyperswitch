from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.stripe_charge_type import StripeChargeType

T = TypeVar("T", bound="PaymentChargeTypeType0")


@_attrs_define
class PaymentChargeTypeType0:
    """
    Attributes:
        stripe (StripeChargeType):
    """

    stripe: StripeChargeType
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        stripe = self.stripe.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "Stripe": stripe,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        stripe = StripeChargeType(d.pop("Stripe"))

        payment_charge_type_type_0 = cls(
            stripe=stripe,
        )

        payment_charge_type_type_0.additional_properties = d
        return payment_charge_type_type_0

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
