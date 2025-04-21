from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="PollConfigResponse")


@_attrs_define
class PollConfigResponse:
    """
    Attributes:
        poll_id (str): Poll Id
        delay_in_secs (int): Interval of the poll
        frequency (int): Frequency of the poll
    """

    poll_id: str
    delay_in_secs: int
    frequency: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        poll_id = self.poll_id

        delay_in_secs = self.delay_in_secs

        frequency = self.frequency

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "poll_id": poll_id,
                "delay_in_secs": delay_in_secs,
                "frequency": frequency,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        poll_id = d.pop("poll_id")

        delay_in_secs = d.pop("delay_in_secs")

        frequency = d.pop("frequency")

        poll_config_response = cls(
            poll_id=poll_id,
            delay_in_secs=delay_in_secs,
            frequency=frequency,
        )

        poll_config_response.additional_properties = d
        return poll_config_response

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
