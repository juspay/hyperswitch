from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.card_redirect_response import CardRedirectResponse


T = TypeVar("T", bound="PaymentMethodDataResponseType13")


@_attrs_define
class PaymentMethodDataResponseType13:
    """
    Attributes:
        card_redirect (CardRedirectResponse):
    """

    card_redirect: "CardRedirectResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card_redirect = self.card_redirect.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card_redirect": card_redirect,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.card_redirect_response import CardRedirectResponse

        d = dict(src_dict)
        card_redirect = CardRedirectResponse.from_dict(d.pop("card_redirect"))

        payment_method_data_response_type_13 = cls(
            card_redirect=card_redirect,
        )

        payment_method_data_response_type_13.additional_properties = d
        return payment_method_data_response_type_13

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
