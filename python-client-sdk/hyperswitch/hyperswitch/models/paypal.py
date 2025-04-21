from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="Paypal")


@_attrs_define
class Paypal:
    """
    Attributes:
        email (str): Email linked with paypal account Example: john.doe@example.com.
        telephone_number (str): mobile number linked to paypal account Example: 16608213349.
        paypal_id (str): id of the paypal account Example: G83KXTJ5EHCQ2.
    """

    email: str
    telephone_number: str
    paypal_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        email = self.email

        telephone_number = self.telephone_number

        paypal_id = self.paypal_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "email": email,
                "telephone_number": telephone_number,
                "paypal_id": paypal_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        email = d.pop("email")

        telephone_number = d.pop("telephone_number")

        paypal_id = d.pop("paypal_id")

        paypal = cls(
            email=email,
            telephone_number=telephone_number,
            paypal_id=paypal_id,
        )

        paypal.additional_properties = d
        return paypal

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
