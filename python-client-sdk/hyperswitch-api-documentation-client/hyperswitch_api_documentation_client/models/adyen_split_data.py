from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.adyen_split_item import AdyenSplitItem


T = TypeVar("T", bound="AdyenSplitData")


@_attrs_define
class AdyenSplitData:
    """Fee information for Split Payments to be charged on the payment being collected for Adyen

    Attributes:
        split_items (list['AdyenSplitItem']): Data for the split items
        store (Union[None, Unset, str]): The store identifier
    """

    split_items: list["AdyenSplitItem"]
    store: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        split_items = []
        for split_items_item_data in self.split_items:
            split_items_item = split_items_item_data.to_dict()
            split_items.append(split_items_item)

        store: Union[None, Unset, str]
        if isinstance(self.store, Unset):
            store = UNSET
        else:
            store = self.store

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "split_items": split_items,
            }
        )
        if store is not UNSET:
            field_dict["store"] = store

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.adyen_split_item import AdyenSplitItem

        d = dict(src_dict)
        split_items = []
        _split_items = d.pop("split_items")
        for split_items_item_data in _split_items:
            split_items_item = AdyenSplitItem.from_dict(split_items_item_data)

            split_items.append(split_items_item)

        def _parse_store(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        store = _parse_store(d.pop("store", UNSET))

        adyen_split_data = cls(
            split_items=split_items,
            store=store,
        )

        return adyen_split_data
