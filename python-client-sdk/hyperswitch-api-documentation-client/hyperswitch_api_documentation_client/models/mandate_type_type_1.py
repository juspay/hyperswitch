from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.mandate_amount_data import MandateAmountData


T = TypeVar("T", bound="MandateTypeType1")


@_attrs_define
class MandateTypeType1:
    """
    Attributes:
        multi_use (Union['MandateAmountData', None]):
    """

    multi_use: Union["MandateAmountData", None]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.mandate_amount_data import MandateAmountData

        multi_use: Union[None, dict[str, Any]]
        if isinstance(self.multi_use, MandateAmountData):
            multi_use = self.multi_use.to_dict()
        else:
            multi_use = self.multi_use

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "multi_use": multi_use,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.mandate_amount_data import MandateAmountData

        d = dict(src_dict)

        def _parse_multi_use(data: object) -> Union["MandateAmountData", None]:
            if data is None:
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                multi_use_type_1 = MandateAmountData.from_dict(data)

                return multi_use_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MandateAmountData", None], data)

        multi_use = _parse_multi_use(d.pop("multi_use"))

        mandate_type_type_1 = cls(
            multi_use=multi_use,
        )

        mandate_type_type_1.additional_properties = d
        return mandate_type_type_1

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
