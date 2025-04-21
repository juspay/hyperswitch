from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.refund_response import RefundResponse


T = TypeVar("T", bound="RefundListResponse")


@_attrs_define
class RefundListResponse:
    """
    Attributes:
        count (int): The number of refunds included in the list
        total_count (int): The total number of refunds in the list
        data (list['RefundResponse']): The List of refund response object
    """

    count: int
    total_count: int
    data: list["RefundResponse"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        count = self.count

        total_count = self.total_count

        data = []
        for data_item_data in self.data:
            data_item = data_item_data.to_dict()
            data.append(data_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "count": count,
                "total_count": total_count,
                "data": data,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.refund_response import RefundResponse

        d = dict(src_dict)
        count = d.pop("count")

        total_count = d.pop("total_count")

        data = []
        _data = d.pop("data")
        for data_item_data in _data:
            data_item = RefundResponse.from_dict(data_item_data)

            data.append(data_item)

        refund_list_response = cls(
            count=count,
            total_count=total_count,
            data=data,
        )

        refund_list_response.additional_properties = d
        return refund_list_response

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
