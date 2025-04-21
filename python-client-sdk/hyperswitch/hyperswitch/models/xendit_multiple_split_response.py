from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.xendit_split_route import XenditSplitRoute


T = TypeVar("T", bound="XenditMultipleSplitResponse")


@_attrs_define
class XenditMultipleSplitResponse:
    """Fee information charged on the payment being collected via xendit

    Attributes:
        split_rule_id (str): Identifier for split rule created for the payment
        name (str): Name to identify split rule. Not required to be unique. Typically based on transaction and/or sub-
            merchant types.
        description (str): Description to identify fee rule
        routes (list['XenditSplitRoute']): Array of objects that define how the platform wants to route the fees and to
            which accounts.
        for_user_id (Union[None, Unset, str]): The sub-account user-id that you want to make this transaction for.
    """

    split_rule_id: str
    name: str
    description: str
    routes: list["XenditSplitRoute"]
    for_user_id: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        split_rule_id = self.split_rule_id

        name = self.name

        description = self.description

        routes = []
        for routes_item_data in self.routes:
            routes_item = routes_item_data.to_dict()
            routes.append(routes_item)

        for_user_id: Union[None, Unset, str]
        if isinstance(self.for_user_id, Unset):
            for_user_id = UNSET
        else:
            for_user_id = self.for_user_id

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "split_rule_id": split_rule_id,
                "name": name,
                "description": description,
                "routes": routes,
            }
        )
        if for_user_id is not UNSET:
            field_dict["for_user_id"] = for_user_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.xendit_split_route import XenditSplitRoute

        d = dict(src_dict)
        split_rule_id = d.pop("split_rule_id")

        name = d.pop("name")

        description = d.pop("description")

        routes = []
        _routes = d.pop("routes")
        for routes_item_data in _routes:
            routes_item = XenditSplitRoute.from_dict(routes_item_data)

            routes.append(routes_item)

        def _parse_for_user_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        for_user_id = _parse_for_user_id(d.pop("for_user_id", UNSET))

        xendit_multiple_split_response = cls(
            split_rule_id=split_rule_id,
            name=name,
            description=description,
            routes=routes,
            for_user_id=for_user_id,
        )

        return xendit_multiple_split_response
