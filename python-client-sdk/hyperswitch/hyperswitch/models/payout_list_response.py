from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.payout_create_response import PayoutCreateResponse


T = TypeVar("T", bound="PayoutListResponse")


@_attrs_define
class PayoutListResponse:
    """
    Attributes:
        size (int): The number of payouts included in the list
        data (list['PayoutCreateResponse']): The list of payouts response objects
        total_count (Union[None, Unset, int]): The total number of available payouts for given constraints
    """

    size: int
    data: list["PayoutCreateResponse"]
    total_count: Union[None, Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        size = self.size

        data = []
        for data_item_data in self.data:
            data_item = data_item_data.to_dict()
            data.append(data_item)

        total_count: Union[None, Unset, int]
        if isinstance(self.total_count, Unset):
            total_count = UNSET
        else:
            total_count = self.total_count

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "size": size,
                "data": data,
            }
        )
        if total_count is not UNSET:
            field_dict["total_count"] = total_count

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.payout_create_response import PayoutCreateResponse

        d = dict(src_dict)
        size = d.pop("size")

        data = []
        _data = d.pop("data")
        for data_item_data in _data:
            data_item = PayoutCreateResponse.from_dict(data_item_data)

            data.append(data_item)

        def _parse_total_count(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        total_count = _parse_total_count(d.pop("total_count", UNSET))

        payout_list_response = cls(
            size=size,
            data=data,
            total_count=total_count,
        )

        payout_list_response.additional_properties = d
        return payout_list_response

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
