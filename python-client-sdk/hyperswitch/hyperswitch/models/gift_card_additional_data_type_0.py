from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.givex_gift_card_additional_data import GivexGiftCardAdditionalData


T = TypeVar("T", bound="GiftCardAdditionalDataType0")


@_attrs_define
class GiftCardAdditionalDataType0:
    """
    Attributes:
        givex (GivexGiftCardAdditionalData):
    """

    givex: "GivexGiftCardAdditionalData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        givex = self.givex.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "givex": givex,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.givex_gift_card_additional_data import GivexGiftCardAdditionalData

        d = dict(src_dict)
        givex = GivexGiftCardAdditionalData.from_dict(d.pop("givex"))

        gift_card_additional_data_type_0 = cls(
            givex=givex,
        )

        gift_card_additional_data_type_0.additional_properties = d
        return gift_card_additional_data_type_0

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
