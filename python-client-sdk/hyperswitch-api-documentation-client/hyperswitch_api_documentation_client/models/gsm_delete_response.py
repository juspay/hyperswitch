from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="GsmDeleteResponse")


@_attrs_define
class GsmDeleteResponse:
    """
    Attributes:
        gsm_rule_delete (bool):
        connector (str): The connector through which payment has gone through
        flow (str): The flow in which the code and message occurred for a connector
        sub_flow (str): The sub_flow in which the code and message occurred  for a connector
        code (str): code received from the connector
    """

    gsm_rule_delete: bool
    connector: str
    flow: str
    sub_flow: str
    code: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        gsm_rule_delete = self.gsm_rule_delete

        connector = self.connector

        flow = self.flow

        sub_flow = self.sub_flow

        code = self.code

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "gsm_rule_delete": gsm_rule_delete,
                "connector": connector,
                "flow": flow,
                "sub_flow": sub_flow,
                "code": code,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        gsm_rule_delete = d.pop("gsm_rule_delete")

        connector = d.pop("connector")

        flow = d.pop("flow")

        sub_flow = d.pop("sub_flow")

        code = d.pop("code")

        gsm_delete_response = cls(
            gsm_rule_delete=gsm_rule_delete,
            connector=connector,
            flow=flow,
            sub_flow=sub_flow,
            code=code,
        )

        gsm_delete_response.additional_properties = d
        return gsm_delete_response

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
