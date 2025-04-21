from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.comparison import Comparison


T = TypeVar("T", bound="IfStatement")


@_attrs_define
class IfStatement:
    """Represents an IF statement with conditions and optional nested IF statements

    ```text
    payment.method = card {
    payment.method.cardtype = (credit, debit) {
    payment.method.network = (amex, rupay, diners)
    }
    }
    ```

        Attributes:
            condition (list['Comparison']):
            nested (Union[None, Unset, list['IfStatement']]):
    """

    condition: list["Comparison"]
    nested: Union[None, Unset, list["IfStatement"]] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        condition = []
        for condition_item_data in self.condition:
            condition_item = condition_item_data.to_dict()
            condition.append(condition_item)

        nested: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.nested, Unset):
            nested = UNSET
        elif isinstance(self.nested, list):
            nested = []
            for nested_type_0_item_data in self.nested:
                nested_type_0_item = nested_type_0_item_data.to_dict()
                nested.append(nested_type_0_item)

        else:
            nested = self.nested

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "condition": condition,
            }
        )
        if nested is not UNSET:
            field_dict["nested"] = nested

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.comparison import Comparison

        d = dict(src_dict)
        condition = []
        _condition = d.pop("condition")
        for condition_item_data in _condition:
            condition_item = Comparison.from_dict(condition_item_data)

            condition.append(condition_item)

        def _parse_nested(data: object) -> Union[None, Unset, list["IfStatement"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                nested_type_0 = []
                _nested_type_0 = data
                for nested_type_0_item_data in _nested_type_0:
                    nested_type_0_item = IfStatement.from_dict(nested_type_0_item_data)

                    nested_type_0.append(nested_type_0_item)

                return nested_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["IfStatement"]], data)

        nested = _parse_nested(d.pop("nested", UNSET))

        if_statement = cls(
            condition=condition,
            nested=nested,
        )

        if_statement.additional_properties = d
        return if_statement

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
