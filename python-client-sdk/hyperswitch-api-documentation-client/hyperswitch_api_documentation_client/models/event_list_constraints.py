import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.event_class import EventClass
from ..models.event_type import EventType
from ..types import UNSET, Unset

T = TypeVar("T", bound="EventListConstraints")


@_attrs_define
class EventListConstraints:
    """The constraints to apply when filtering events.

    Attributes:
        created_after (Union[None, Unset, datetime.datetime]): Filter events created after the specified time.
        created_before (Union[None, Unset, datetime.datetime]): Filter events created before the specified time.
        limit (Union[None, Unset, int]): Include at most the specified number of events.
        offset (Union[None, Unset, int]): Include events after the specified offset.
        object_id (Union[None, Unset, str]): Filter all events associated with the specified object identifier (Payment
            Intent ID,
            Refund ID, etc.)
        profile_id (Union[None, Unset, str]): Filter all events associated with the specified business profile ID.
        event_classes (Union[None, Unset, list[EventClass]]): Filter events by their class.
        event_types (Union[None, Unset, list[EventType]]): Filter events by their type.
        is_delivered (Union[None, Unset, bool]): Filter all events by `is_overall_delivery_successful` field of the
            event.
    """

    created_after: Union[None, Unset, datetime.datetime] = UNSET
    created_before: Union[None, Unset, datetime.datetime] = UNSET
    limit: Union[None, Unset, int] = UNSET
    offset: Union[None, Unset, int] = UNSET
    object_id: Union[None, Unset, str] = UNSET
    profile_id: Union[None, Unset, str] = UNSET
    event_classes: Union[None, Unset, list[EventClass]] = UNSET
    event_types: Union[None, Unset, list[EventType]] = UNSET
    is_delivered: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        created_after: Union[None, Unset, str]
        if isinstance(self.created_after, Unset):
            created_after = UNSET
        elif isinstance(self.created_after, datetime.datetime):
            created_after = self.created_after.isoformat()
        else:
            created_after = self.created_after

        created_before: Union[None, Unset, str]
        if isinstance(self.created_before, Unset):
            created_before = UNSET
        elif isinstance(self.created_before, datetime.datetime):
            created_before = self.created_before.isoformat()
        else:
            created_before = self.created_before

        limit: Union[None, Unset, int]
        if isinstance(self.limit, Unset):
            limit = UNSET
        else:
            limit = self.limit

        offset: Union[None, Unset, int]
        if isinstance(self.offset, Unset):
            offset = UNSET
        else:
            offset = self.offset

        object_id: Union[None, Unset, str]
        if isinstance(self.object_id, Unset):
            object_id = UNSET
        else:
            object_id = self.object_id

        profile_id: Union[None, Unset, str]
        if isinstance(self.profile_id, Unset):
            profile_id = UNSET
        else:
            profile_id = self.profile_id

        event_classes: Union[None, Unset, list[str]]
        if isinstance(self.event_classes, Unset):
            event_classes = UNSET
        elif isinstance(self.event_classes, list):
            event_classes = []
            for event_classes_type_0_item_data in self.event_classes:
                event_classes_type_0_item = event_classes_type_0_item_data.value
                event_classes.append(event_classes_type_0_item)

        else:
            event_classes = self.event_classes

        event_types: Union[None, Unset, list[str]]
        if isinstance(self.event_types, Unset):
            event_types = UNSET
        elif isinstance(self.event_types, list):
            event_types = []
            for event_types_type_0_item_data in self.event_types:
                event_types_type_0_item = event_types_type_0_item_data.value
                event_types.append(event_types_type_0_item)

        else:
            event_types = self.event_types

        is_delivered: Union[None, Unset, bool]
        if isinstance(self.is_delivered, Unset):
            is_delivered = UNSET
        else:
            is_delivered = self.is_delivered

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if created_after is not UNSET:
            field_dict["created_after"] = created_after
        if created_before is not UNSET:
            field_dict["created_before"] = created_before
        if limit is not UNSET:
            field_dict["limit"] = limit
        if offset is not UNSET:
            field_dict["offset"] = offset
        if object_id is not UNSET:
            field_dict["object_id"] = object_id
        if profile_id is not UNSET:
            field_dict["profile_id"] = profile_id
        if event_classes is not UNSET:
            field_dict["event_classes"] = event_classes
        if event_types is not UNSET:
            field_dict["event_types"] = event_types
        if is_delivered is not UNSET:
            field_dict["is_delivered"] = is_delivered

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_created_after(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                created_after_type_0 = isoparse(data)

                return created_after_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        created_after = _parse_created_after(d.pop("created_after", UNSET))

        def _parse_created_before(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                created_before_type_0 = isoparse(data)

                return created_before_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        created_before = _parse_created_before(d.pop("created_before", UNSET))

        def _parse_limit(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        limit = _parse_limit(d.pop("limit", UNSET))

        def _parse_offset(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        offset = _parse_offset(d.pop("offset", UNSET))

        def _parse_object_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        object_id = _parse_object_id(d.pop("object_id", UNSET))

        def _parse_profile_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        profile_id = _parse_profile_id(d.pop("profile_id", UNSET))

        def _parse_event_classes(data: object) -> Union[None, Unset, list[EventClass]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                event_classes_type_0 = []
                _event_classes_type_0 = data
                for event_classes_type_0_item_data in _event_classes_type_0:
                    event_classes_type_0_item = EventClass(event_classes_type_0_item_data)

                    event_classes_type_0.append(event_classes_type_0_item)

                return event_classes_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[EventClass]], data)

        event_classes = _parse_event_classes(d.pop("event_classes", UNSET))

        def _parse_event_types(data: object) -> Union[None, Unset, list[EventType]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                event_types_type_0 = []
                _event_types_type_0 = data
                for event_types_type_0_item_data in _event_types_type_0:
                    event_types_type_0_item = EventType(event_types_type_0_item_data)

                    event_types_type_0.append(event_types_type_0_item)

                return event_types_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[EventType]], data)

        event_types = _parse_event_types(d.pop("event_types", UNSET))

        def _parse_is_delivered(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_delivered = _parse_is_delivered(d.pop("is_delivered", UNSET))

        event_list_constraints = cls(
            created_after=created_after,
            created_before=created_before,
            limit=limit,
            offset=offset,
            object_id=object_id,
            profile_id=profile_id,
            event_classes=event_classes,
            event_types=event_types,
            is_delivered=is_delivered,
        )

        event_list_constraints.additional_properties = d
        return event_list_constraints

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
