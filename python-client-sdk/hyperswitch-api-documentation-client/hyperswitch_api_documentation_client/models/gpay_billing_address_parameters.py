from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.gpay_billing_address_format import GpayBillingAddressFormat

T = TypeVar("T", bound="GpayBillingAddressParameters")


@_attrs_define
class GpayBillingAddressParameters:
    """
    Attributes:
        phone_number_required (bool): Is billing phone number required
        format_ (GpayBillingAddressFormat):
    """

    phone_number_required: bool
    format_: GpayBillingAddressFormat
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        phone_number_required = self.phone_number_required

        format_ = self.format_.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "phone_number_required": phone_number_required,
                "format": format_,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        phone_number_required = d.pop("phone_number_required")

        format_ = GpayBillingAddressFormat(d.pop("format"))

        gpay_billing_address_parameters = cls(
            phone_number_required=phone_number_required,
            format_=format_,
        )

        gpay_billing_address_parameters.additional_properties = d
        return gpay_billing_address_parameters

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
