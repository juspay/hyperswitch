from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.payment_method import PaymentMethod
from ..models.payment_method_type import PaymentMethodType

T = TypeVar("T", bound="EnabledPaymentMethod")


@_attrs_define
class EnabledPaymentMethod:
    """Object for EnabledPaymentMethod

    Attributes:
        payment_method (PaymentMethod): Indicates the type of payment method. Eg: 'card', 'wallet', etc.
        payment_method_types (list[PaymentMethodType]): An array of associated payment method types
    """

    payment_method: PaymentMethod
    payment_method_types: list[PaymentMethodType]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payment_method = self.payment_method.value

        payment_method_types = []
        for payment_method_types_item_data in self.payment_method_types:
            payment_method_types_item = payment_method_types_item_data.value
            payment_method_types.append(payment_method_types_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_method": payment_method,
                "payment_method_types": payment_method_types,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        payment_method = PaymentMethod(d.pop("payment_method"))

        payment_method_types = []
        _payment_method_types = d.pop("payment_method_types")
        for payment_method_types_item_data in _payment_method_types:
            payment_method_types_item = PaymentMethodType(payment_method_types_item_data)

            payment_method_types.append(payment_method_types_item)

        enabled_payment_method = cls(
            payment_method=payment_method,
            payment_method_types=payment_method_types,
        )

        enabled_payment_method.additional_properties = d
        return enabled_payment_method

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
