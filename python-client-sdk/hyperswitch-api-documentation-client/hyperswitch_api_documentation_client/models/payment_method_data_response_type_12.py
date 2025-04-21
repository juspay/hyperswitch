from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.gift_card_response import GiftCardResponse


T = TypeVar("T", bound="PaymentMethodDataResponseType12")


@_attrs_define
class PaymentMethodDataResponseType12:
    """
    Attributes:
        gift_card (GiftCardResponse):
    """

    gift_card: "GiftCardResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        gift_card = self.gift_card.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "gift_card": gift_card,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.gift_card_response import GiftCardResponse

        d = dict(src_dict)
        gift_card = GiftCardResponse.from_dict(d.pop("gift_card"))

        payment_method_data_response_type_12 = cls(
            gift_card=gift_card,
        )

        payment_method_data_response_type_12.additional_properties = d
        return payment_method_data_response_type_12

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
