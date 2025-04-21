import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from dateutil.parser import isoparse

from ..types import UNSET, Unset

T = TypeVar("T", bound="PaymentListConstraints")


@_attrs_define
class PaymentListConstraints:
    """
    Attributes:
        customer_id (Union[None, Unset, str]): The identifier for customer Example: cus_y3oqhf46pyzuxjbcn2giaqnb44.
        starting_after (Union[None, Unset, str]): A cursor for use in pagination, fetch the next list after some object
            Example: pay_fafa124123.
        ending_before (Union[None, Unset, str]): A cursor for use in pagination, fetch the previous list before some
            object Example: pay_fafa124123.
        limit (Union[Unset, int]): limit on the number of objects to return Default: 10.
        created (Union[None, Unset, datetime.datetime]): The time at which payment is created Example:
            2022-09-10T10:11:12Z.
        created_lt (Union[None, Unset, datetime.datetime]): Time less than the payment created time Example:
            2022-09-10T10:11:12Z.
        created_gt (Union[None, Unset, datetime.datetime]): Time greater than the payment created time Example:
            2022-09-10T10:11:12Z.
        created_lte (Union[None, Unset, datetime.datetime]): Time less than or equals to the payment created time
            Example: 2022-09-10T10:11:12Z.
        created_gte (Union[None, Unset, datetime.datetime]): Time greater than or equals to the payment created time
            Example: 2022-09-10T10:11:12Z.
    """

    customer_id: Union[None, Unset, str] = UNSET
    starting_after: Union[None, Unset, str] = UNSET
    ending_before: Union[None, Unset, str] = UNSET
    limit: Union[Unset, int] = 10
    created: Union[None, Unset, datetime.datetime] = UNSET
    created_lt: Union[None, Unset, datetime.datetime] = UNSET
    created_gt: Union[None, Unset, datetime.datetime] = UNSET
    created_lte: Union[None, Unset, datetime.datetime] = UNSET
    created_gte: Union[None, Unset, datetime.datetime] = UNSET

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

        created_lt: Union[None, Unset, str]
        if isinstance(self.created_lt, Unset):
            created_lt = UNSET
        elif isinstance(self.created_lt, datetime.datetime):
            created_lt = self.created_lt.isoformat()
        else:
            created_lt = self.created_lt

        created_gt: Union[None, Unset, str]
        if isinstance(self.created_gt, Unset):
            created_gt = UNSET
        elif isinstance(self.created_gt, datetime.datetime):
            created_gt = self.created_gt.isoformat()
        else:
            created_gt = self.created_gt

        created_lte: Union[None, Unset, str]
        if isinstance(self.created_lte, Unset):
            created_lte = UNSET
        elif isinstance(self.created_lte, datetime.datetime):
            created_lte = self.created_lte.isoformat()
        else:
            created_lte = self.created_lte

        created_gte: Union[None, Unset, str]
        if isinstance(self.created_gte, Unset):
            created_gte = UNSET
        elif isinstance(self.created_gte, datetime.datetime):
            created_gte = self.created_gte.isoformat()
        else:
            created_gte = self.created_gte

        field_dict: dict[str, Any] = {}
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
        if created_lt is not UNSET:
            field_dict["created.lt"] = created_lt
        if created_gt is not UNSET:
            field_dict["created.gt"] = created_gt
        if created_lte is not UNSET:
            field_dict["created.lte"] = created_lte
        if created_gte is not UNSET:
            field_dict["created.gte"] = created_gte

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

        def _parse_created_lt(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                created_lt_type_0 = isoparse(data)

                return created_lt_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        created_lt = _parse_created_lt(d.pop("created.lt", UNSET))

        def _parse_created_gt(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                created_gt_type_0 = isoparse(data)

                return created_gt_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        created_gt = _parse_created_gt(d.pop("created.gt", UNSET))

        def _parse_created_lte(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                created_lte_type_0 = isoparse(data)

                return created_lte_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        created_lte = _parse_created_lte(d.pop("created.lte", UNSET))

        def _parse_created_gte(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                created_gte_type_0 = isoparse(data)

                return created_gte_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        created_gte = _parse_created_gte(d.pop("created.gte", UNSET))

        payment_list_constraints = cls(
            customer_id=customer_id,
            starting_after=starting_after,
            ending_before=ending_before,
            limit=limit,
            created=created,
            created_lt=created_lt,
            created_gt=created_gt,
            created_lte=created_lte,
            created_gte=created_gte,
        )

        return payment_list_constraints
