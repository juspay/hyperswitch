import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..types import UNSET, Unset

T = TypeVar("T", bound="PayoutListConstraints")


@_attrs_define
class PayoutListConstraints:
    """
    Attributes:
        customer_id (Union[None, Unset, str]): The identifier for customer Example: cus_y3oqhf46pyzuxjbcn2giaqnb44.
        starting_after (Union[None, Unset, str]): A cursor for use in pagination, fetch the next list after some object
            Example: pay_fafa124123.
        ending_before (Union[None, Unset, str]): A cursor for use in pagination, fetch the previous list before some
            object Example: pay_fafa124123.
        limit (Union[Unset, int]): limit on the number of objects to return Default: 10.
        created (Union[None, Unset, datetime.datetime]): The time at which payout is created Example:
            2022-09-10T10:11:12Z.
    """

    customer_id: Union[None, Unset, str] = UNSET
    starting_after: Union[None, Unset, str] = UNSET
    ending_before: Union[None, Unset, str] = UNSET
    limit: Union[Unset, int] = 10
    created: Union[None, Unset, datetime.datetime] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        customer_id: Union[None, Unset, str]
        if isinstance(self.customer_id, Unset):
            customer_id = UNSET
        else:
            customer_id = self.customer_id

        starting_after: Union[None, Unset, str]
        if isinstance(self.starting_after, Unset):
            starting_after = UNSET
        else:
            starting_after = self.starting_after

        ending_before: Union[None, Unset, str]
        if isinstance(self.ending_before, Unset):
            ending_before = UNSET
        else:
            ending_before = self.ending_before

        limit = self.limit

        created: Union[None, Unset, str]
        if isinstance(self.created, Unset):
            created = UNSET
        elif isinstance(self.created, datetime.datetime):
            created = self.created.isoformat()
        else:
            created = self.created

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if customer_id is not UNSET:
            field_dict["customer_id"] = customer_id
        if starting_after is not UNSET:
            field_dict["starting_after"] = starting_after
        if ending_before is not UNSET:
            field_dict["ending_before"] = ending_before
        if limit is not UNSET:
            field_dict["limit"] = limit
        if created is not UNSET:
            field_dict["created"] = created

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_customer_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        customer_id = _parse_customer_id(d.pop("customer_id", UNSET))

        def _parse_starting_after(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        starting_after = _parse_starting_after(d.pop("starting_after", UNSET))

        def _parse_ending_before(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        ending_before = _parse_ending_before(d.pop("ending_before", UNSET))

        limit = d.pop("limit", UNSET)

        def _parse_created(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                created_type_0 = isoparse(data)

                return created_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        created = _parse_created(d.pop("created", UNSET))

        payout_list_constraints = cls(
            customer_id=customer_id,
            starting_after=starting_after,
            ending_before=ending_before,
            limit=limit,
            created=created,
        )

        payout_list_constraints.additional_properties = d
        return payout_list_constraints

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
