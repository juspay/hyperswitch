from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.open_banking_response import OpenBankingResponse


T = TypeVar("T", bound="PaymentMethodDataResponseType15")


@_attrs_define
class PaymentMethodDataResponseType15:
    """
    Attributes:
        open_banking (OpenBankingResponse):
    """

    open_banking: "OpenBankingResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        open_banking = self.open_banking.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "open_banking": open_banking,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.open_banking_response import OpenBankingResponse

        d = dict(src_dict)
        open_banking = OpenBankingResponse.from_dict(d.pop("open_banking"))

        payment_method_data_response_type_15 = cls(
            open_banking=open_banking,
        )

        payment_method_data_response_type_15.additional_properties = d
        return payment_method_data_response_type_15

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
