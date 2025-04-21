from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.paylater_response import PaylaterResponse


T = TypeVar("T", bound="PaymentMethodDataResponseType3")


@_attrs_define
class PaymentMethodDataResponseType3:
    """
    Attributes:
        pay_later (PaylaterResponse):
    """

    pay_later: "PaylaterResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        pay_later = self.pay_later.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "pay_later": pay_later,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.paylater_response import PaylaterResponse

        d = dict(src_dict)
        pay_later = PaylaterResponse.from_dict(d.pop("pay_later"))

        payment_method_data_response_type_3 = cls(
            pay_later=pay_later,
        )

        payment_method_data_response_type_3.additional_properties = d
        return payment_method_data_response_type_3

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
