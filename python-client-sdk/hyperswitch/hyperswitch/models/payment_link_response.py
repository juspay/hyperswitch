from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="PaymentLinkResponse")


@_attrs_define
class PaymentLinkResponse:
    """
    Attributes:
        link (str): URL for rendering the open payment link
        payment_link_id (str): Identifier for the payment link
        secure_link (Union[None, Unset, str]): URL for rendering the secure payment link
    """

    link: str
    payment_link_id: str
    secure_link: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        link = self.link

        payment_link_id = self.payment_link_id

        secure_link: Union[None, Unset, str]
        if isinstance(self.secure_link, Unset):
            secure_link = UNSET
        else:
            secure_link = self.secure_link

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "link": link,
                "payment_link_id": payment_link_id,
            }
        )
        if secure_link is not UNSET:
            field_dict["secure_link"] = secure_link

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        link = d.pop("link")

        payment_link_id = d.pop("payment_link_id")

        def _parse_secure_link(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        secure_link = _parse_secure_link(d.pop("secure_link", UNSET))

        payment_link_response = cls(
            link=link,
            payment_link_id=payment_link_id,
            secure_link=secure_link,
        )

        payment_link_response.additional_properties = d
        return payment_link_response

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
