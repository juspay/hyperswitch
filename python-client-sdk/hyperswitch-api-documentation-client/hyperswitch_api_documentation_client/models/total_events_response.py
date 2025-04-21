from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.event_list_item_response import EventListItemResponse


T = TypeVar("T", bound="TotalEventsResponse")


@_attrs_define
class TotalEventsResponse:
    """The response body of list initial delivery attempts api call.

    Attributes:
        events (list['EventListItemResponse']): The list of events
        total_count (int): Count of total events
    """

    events: list["EventListItemResponse"]
    total_count: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        events = []
        for events_item_data in self.events:
            events_item = events_item_data.to_dict()
            events.append(events_item)

        total_count = self.total_count

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "events": events,
                "total_count": total_count,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.event_list_item_response import EventListItemResponse

        d = dict(src_dict)
        events = []
        _events = d.pop("events")
        for events_item_data in _events:
            events_item = EventListItemResponse.from_dict(events_item_data)

            events.append(events_item)

        total_count = d.pop("total_count")

        total_events_response = cls(
            events=events,
            total_count=total_count,
        )

        total_events_response.additional_properties = d
        return total_events_response

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
