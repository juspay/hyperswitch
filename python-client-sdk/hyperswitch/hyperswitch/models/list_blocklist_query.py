from collections.abc import Mapping
from typing import Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.blocklist_data_kind import BlocklistDataKind
from ..types import UNSET, Unset

T = TypeVar("T", bound="ListBlocklistQuery")


@_attrs_define
class ListBlocklistQuery:
    """
    Attributes:
        data_kind (BlocklistDataKind):
        limit (Union[Unset, int]):
        offset (Union[Unset, int]):
    """

    data_kind: BlocklistDataKind
    limit: Union[Unset, int] = UNSET
    offset: Union[Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        data_kind = self.data_kind.value

        limit = self.limit

        offset = self.offset

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "data_kind": data_kind,
            }
        )
        if limit is not UNSET:
            field_dict["limit"] = limit
        if offset is not UNSET:
            field_dict["offset"] = offset

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        data_kind = BlocklistDataKind(d.pop("data_kind"))

        limit = d.pop("limit", UNSET)

        offset = d.pop("offset", UNSET)

        list_blocklist_query = cls(
            data_kind=data_kind,
            limit=limit,
            offset=offset,
        )

        list_blocklist_query.additional_properties = d
        return list_blocklist_query

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
