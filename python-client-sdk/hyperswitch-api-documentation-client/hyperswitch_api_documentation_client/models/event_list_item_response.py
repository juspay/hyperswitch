import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.event_class import EventClass
from ..models.event_type import EventType
from ..types import UNSET, Unset

T = TypeVar("T", bound="EventListItemResponse")


@_attrs_define
class EventListItemResponse:
    """The response body for each item when listing events.

    Attributes:
        event_id (str): The identifier for the Event. Example: evt_018e31720d1b7a2b82677d3032cab959.
        merchant_id (str): The identifier for the Merchant Account. Example: y3oqhf46pyzuxjbcn2giaqnb44.
        profile_id (str): The identifier for the Business Profile. Example: SqB0zwDGR5wHppWf0bx7GKr1f2.
        object_id (str): The identifier for the object (Payment Intent ID, Refund ID, etc.) Example:
            QHrfd5LUDdZaKtAjdJmMu0dMa1.
        event_type (EventType):
        event_class (EventClass):
        initial_attempt_id (str): The identifier for the initial delivery attempt. This will be the same as `event_id`
            for
            the initial delivery attempt. Example: evt_018e31720d1b7a2b82677d3032cab959.
        created (datetime.datetime): Time at which the event was created. Example: 2022-09-10T10:11:12Z.
        is_delivery_successful (Union[None, Unset, bool]): Indicates whether the webhook was ultimately delivered or
            not.
    """

    event_id: str
    merchant_id: str
    profile_id: str
    object_id: str
    event_type: EventType
    event_class: EventClass
    initial_attempt_id: str
    created: datetime.datetime
    is_delivery_successful: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        event_id = self.event_id

        merchant_id = self.merchant_id

        profile_id = self.profile_id

        object_id = self.object_id

        event_type = self.event_type.value

        event_class = self.event_class.value

        initial_attempt_id = self.initial_attempt_id

        created = self.created.isoformat()

        is_delivery_successful: Union[None, Unset, bool]
        if isinstance(self.is_delivery_successful, Unset):
            is_delivery_successful = UNSET
        else:
            is_delivery_successful = self.is_delivery_successful

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "event_id": event_id,
                "merchant_id": merchant_id,
                "profile_id": profile_id,
                "object_id": object_id,
                "event_type": event_type,
                "event_class": event_class,
                "initial_attempt_id": initial_attempt_id,
                "created": created,
            }
        )
        if is_delivery_successful is not UNSET:
            field_dict["is_delivery_successful"] = is_delivery_successful

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        event_id = d.pop("event_id")

        merchant_id = d.pop("merchant_id")

        profile_id = d.pop("profile_id")

        object_id = d.pop("object_id")

        event_type = EventType(d.pop("event_type"))

        event_class = EventClass(d.pop("event_class"))

        initial_attempt_id = d.pop("initial_attempt_id")

        created = isoparse(d.pop("created"))

        def _parse_is_delivery_successful(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_delivery_successful = _parse_is_delivery_successful(d.pop("is_delivery_successful", UNSET))

        event_list_item_response = cls(
            event_id=event_id,
            merchant_id=merchant_id,
            profile_id=profile_id,
            object_id=object_id,
            event_type=event_type,
            event_class=event_class,
            initial_attempt_id=initial_attempt_id,
            created=created,
            is_delivery_successful=is_delivery_successful,
        )

        event_list_item_response.additional_properties = d
        return event_list_item_response

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
