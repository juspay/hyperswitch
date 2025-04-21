from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.element_position import ElementPosition
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.element_size_type_0 import ElementSizeType0
    from ..models.element_size_type_1 import ElementSizeType1
    from ..models.element_size_type_2 import ElementSizeType2


T = TypeVar("T", bound="PaymentLinkBackgroundImageConfig")


@_attrs_define
class PaymentLinkBackgroundImageConfig:
    """
    Attributes:
        url (str): URL of the image Example: https://hyperswitch.io/favicon.ico.
        position (Union[ElementPosition, None, Unset]):
        size (Union['ElementSizeType0', 'ElementSizeType1', 'ElementSizeType2', None, Unset]):
    """

    url: str
    position: Union[ElementPosition, None, Unset] = UNSET
    size: Union["ElementSizeType0", "ElementSizeType1", "ElementSizeType2", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.element_size_type_0 import ElementSizeType0
        from ..models.element_size_type_1 import ElementSizeType1
        from ..models.element_size_type_2 import ElementSizeType2

        url = self.url

        position: Union[None, Unset, str]
        if isinstance(self.position, Unset):
            position = UNSET
        elif isinstance(self.position, ElementPosition):
            position = self.position.value
        else:
            position = self.position

        size: Union[None, Unset, dict[str, Any]]
        if isinstance(self.size, Unset):
            size = UNSET
        elif isinstance(self.size, ElementSizeType0):
            size = self.size.to_dict()
        elif isinstance(self.size, ElementSizeType1):
            size = self.size.to_dict()
        elif isinstance(self.size, ElementSizeType2):
            size = self.size.to_dict()
        else:
            size = self.size

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "url": url,
            }
        )
        if position is not UNSET:
            field_dict["position"] = position
        if size is not UNSET:
            field_dict["size"] = size

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.element_size_type_0 import ElementSizeType0
        from ..models.element_size_type_1 import ElementSizeType1
        from ..models.element_size_type_2 import ElementSizeType2

        d = dict(src_dict)
        url = d.pop("url")

        def _parse_position(data: object) -> Union[ElementPosition, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                position_type_1 = ElementPosition(data)

                return position_type_1
            except:  # noqa: E722
                pass
            return cast(Union[ElementPosition, None, Unset], data)

        position = _parse_position(d.pop("position", UNSET))

        def _parse_size(data: object) -> Union["ElementSizeType0", "ElementSizeType1", "ElementSizeType2", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_element_size_type_0 = ElementSizeType0.from_dict(data)

                return componentsschemas_element_size_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_element_size_type_1 = ElementSizeType1.from_dict(data)

                return componentsschemas_element_size_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_element_size_type_2 = ElementSizeType2.from_dict(data)

                return componentsschemas_element_size_type_2
            except:  # noqa: E722
                pass
            return cast(Union["ElementSizeType0", "ElementSizeType1", "ElementSizeType2", None, Unset], data)

        size = _parse_size(d.pop("size", UNSET))

        payment_link_background_image_config = cls(
            url=url,
            position=position,
            size=size,
        )

        payment_link_background_image_config.additional_properties = d
        return payment_link_background_image_config

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
