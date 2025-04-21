import datetime
from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.blocklist_data_kind import BlocklistDataKind

T = TypeVar("T", bound="BlocklistResponse")


@_attrs_define
class BlocklistResponse:
    """
    Attributes:
        fingerprint_id (str):
        data_kind (BlocklistDataKind):
        created_at (datetime.datetime):
    """

    fingerprint_id: str
    data_kind: BlocklistDataKind
    created_at: datetime.datetime
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        fingerprint_id = self.fingerprint_id

        data_kind = self.data_kind.value

        created_at = self.created_at.isoformat()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "fingerprint_id": fingerprint_id,
                "data_kind": data_kind,
                "created_at": created_at,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        fingerprint_id = d.pop("fingerprint_id")

        data_kind = BlocklistDataKind(d.pop("data_kind"))

        created_at = isoparse(d.pop("created_at"))

        blocklist_response = cls(
            fingerprint_id=fingerprint_id,
            data_kind=data_kind,
            created_at=created_at,
        )

        blocklist_response.additional_properties = d
        return blocklist_response

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
