from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.card_additional_data import CardAdditionalData


T = TypeVar("T", bound="PayoutMethodDataResponseType0")


@_attrs_define
class PayoutMethodDataResponseType0:
    """
    Attributes:
        card (CardAdditionalData): Masked payout method details for card payout method
    """

    card: "CardAdditionalData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card = self.card.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card": card,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.card_additional_data import CardAdditionalData

        d = dict(src_dict)
        card = CardAdditionalData.from_dict(d.pop("card"))

        payout_method_data_response_type_0 = cls(
            card=card,
        )

        payout_method_data_response_type_0.additional_properties = d
        return payout_method_data_response_type_0

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
