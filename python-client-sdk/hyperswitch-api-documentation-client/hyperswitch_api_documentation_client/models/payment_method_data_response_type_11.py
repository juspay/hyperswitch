from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.voucher_response import VoucherResponse


T = TypeVar("T", bound="PaymentMethodDataResponseType11")


@_attrs_define
class PaymentMethodDataResponseType11:
    """
    Attributes:
        voucher (VoucherResponse):
    """

    voucher: "VoucherResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        voucher = self.voucher.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "voucher": voucher,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.voucher_response import VoucherResponse

        d = dict(src_dict)
        voucher = VoucherResponse.from_dict(d.pop("voucher"))

        payment_method_data_response_type_11 = cls(
            voucher=voucher,
        )

        payment_method_data_response_type_11.additional_properties = d
        return payment_method_data_response_type_11

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
