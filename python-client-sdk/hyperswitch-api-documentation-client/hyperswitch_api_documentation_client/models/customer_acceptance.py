import datetime
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from dateutil.parser import isoparse

from ..models.acceptance_type import AcceptanceType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.online_mandate import OnlineMandate


T = TypeVar("T", bound="CustomerAcceptance")


@_attrs_define
class CustomerAcceptance:
    """This "CustomerAcceptance" object is passed during Payments-Confirm request, it enlists the type, time, and mode of
    acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually
    passed by the SDK or client.

        Attributes:
            acceptance_type (AcceptanceType): This is used to indicate if the mandate was accepted online or offline
            accepted_at (Union[None, Unset, datetime.datetime]): Specifying when the customer acceptance was provided
                Example: 2022-09-10T10:11:12Z.
            online (Union['OnlineMandate', None, Unset]):
    """

    acceptance_type: AcceptanceType
    accepted_at: Union[None, Unset, datetime.datetime] = UNSET
    online: Union["OnlineMandate", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.online_mandate import OnlineMandate

        acceptance_type = self.acceptance_type.value

        accepted_at: Union[None, Unset, str]
        if isinstance(self.accepted_at, Unset):
            accepted_at = UNSET
        elif isinstance(self.accepted_at, datetime.datetime):
            accepted_at = self.accepted_at.isoformat()
        else:
            accepted_at = self.accepted_at

        online: Union[None, Unset, dict[str, Any]]
        if isinstance(self.online, Unset):
            online = UNSET
        elif isinstance(self.online, OnlineMandate):
            online = self.online.to_dict()
        else:
            online = self.online

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "acceptance_type": acceptance_type,
            }
        )
        if accepted_at is not UNSET:
            field_dict["accepted_at"] = accepted_at
        if online is not UNSET:
            field_dict["online"] = online

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.online_mandate import OnlineMandate

        d = dict(src_dict)
        acceptance_type = AcceptanceType(d.pop("acceptance_type"))

        def _parse_accepted_at(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                accepted_at_type_0 = isoparse(data)

                return accepted_at_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        accepted_at = _parse_accepted_at(d.pop("accepted_at", UNSET))

        def _parse_online(data: object) -> Union["OnlineMandate", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                online_type_1 = OnlineMandate.from_dict(data)

                return online_type_1
            except:  # noqa: E722
                pass
            return cast(Union["OnlineMandate", None, Unset], data)

        online = _parse_online(d.pop("online", UNSET))

        customer_acceptance = cls(
            acceptance_type=acceptance_type,
            accepted_at=accepted_at,
            online=online,
        )

        return customer_acceptance
