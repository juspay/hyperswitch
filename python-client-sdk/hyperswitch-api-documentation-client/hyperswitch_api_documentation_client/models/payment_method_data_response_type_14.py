from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.card_token_response import CardTokenResponse


T = TypeVar("T", bound="PaymentMethodDataResponseType14")


@_attrs_define
class PaymentMethodDataResponseType14:
    """
    Attributes:
        card_token (CardTokenResponse):
    """

    card_token: "CardTokenResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card_token = self.card_token.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card_token": card_token,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.card_token_response import CardTokenResponse

        d = dict(src_dict)
        card_token = CardTokenResponse.from_dict(d.pop("card_token"))

        payment_method_data_response_type_14 = cls(
            card_token=card_token,
        )

        payment_method_data_response_type_14.additional_properties = d
        return payment_method_data_response_type_14

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
