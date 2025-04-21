from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="PaymentProcessingDetails")


@_attrs_define
class PaymentProcessingDetails:
    """
    Attributes:
        payment_processing_certificate (str):
        payment_processing_certificate_key (str):
    """

    payment_processing_certificate: str
    payment_processing_certificate_key: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payment_processing_certificate = self.payment_processing_certificate

        payment_processing_certificate_key = self.payment_processing_certificate_key

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_processing_certificate": payment_processing_certificate,
                "payment_processing_certificate_key": payment_processing_certificate_key,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        payment_processing_certificate = d.pop("payment_processing_certificate")

        payment_processing_certificate_key = d.pop("payment_processing_certificate_key")

        payment_processing_details = cls(
            payment_processing_certificate=payment_processing_certificate,
            payment_processing_certificate_key=payment_processing_certificate_key,
        )

        payment_processing_details.additional_properties = d
        return payment_processing_details

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
