from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.routing_dictionary_record import RoutingDictionaryRecord


T = TypeVar("T", bound="RoutingDictionary")


@_attrs_define
class RoutingDictionary:
    """
    Attributes:
        merchant_id (str):
        records (list['RoutingDictionaryRecord']):
        active_id (Union[None, Unset, str]):
    """

    merchant_id: str
    records: list["RoutingDictionaryRecord"]
    active_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        merchant_id = self.merchant_id

        records = []
        for records_item_data in self.records:
            records_item = records_item_data.to_dict()
            records.append(records_item)

        active_id: Union[None, Unset, str]
        if isinstance(self.active_id, Unset):
            active_id = UNSET
        else:
            active_id = self.active_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "merchant_id": merchant_id,
                "records": records,
            }
        )
        if active_id is not UNSET:
            field_dict["active_id"] = active_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.routing_dictionary_record import RoutingDictionaryRecord

        d = dict(src_dict)
        merchant_id = d.pop("merchant_id")

        records = []
        _records = d.pop("records")
        for records_item_data in _records:
            records_item = RoutingDictionaryRecord.from_dict(records_item_data)

            records.append(records_item)

        def _parse_active_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        active_id = _parse_active_id(d.pop("active_id", UNSET))

        routing_dictionary = cls(
            merchant_id=merchant_id,
            records=records,
            active_id=active_id,
        )

        routing_dictionary.additional_properties = d
        return routing_dictionary

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
