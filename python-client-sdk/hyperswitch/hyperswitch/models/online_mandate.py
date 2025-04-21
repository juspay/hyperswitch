from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define

T = TypeVar("T", bound="OnlineMandate")


@_attrs_define
class OnlineMandate:
    """
    Attributes:
        ip_address (str): Ip address of the customer machine from which the mandate was created Example: 123.32.25.123.
        user_agent (str): The user-agent of the customer's browser
    """

    ip_address: str
    user_agent: str

    def to_dict(self) -> dict[str, Any]:
        ip_address = self.ip_address

        user_agent = self.user_agent

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "ip_address": ip_address,
                "user_agent": user_agent,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        ip_address = d.pop("ip_address")

        user_agent = d.pop("user_agent")

        online_mandate = cls(
            ip_address=ip_address,
            user_agent=user_agent,
        )

        return online_mandate
