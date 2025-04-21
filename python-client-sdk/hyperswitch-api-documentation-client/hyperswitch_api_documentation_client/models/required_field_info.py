from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.field_type_type_0 import FieldTypeType0
from ..models.field_type_type_1 import FieldTypeType1
from ..models.field_type_type_2 import FieldTypeType2
from ..models.field_type_type_3 import FieldTypeType3
from ..models.field_type_type_4 import FieldTypeType4
from ..models.field_type_type_5 import FieldTypeType5
from ..models.field_type_type_6 import FieldTypeType6
from ..models.field_type_type_7 import FieldTypeType7
from ..models.field_type_type_8 import FieldTypeType8
from ..models.field_type_type_11 import FieldTypeType11
from ..models.field_type_type_12 import FieldTypeType12
from ..models.field_type_type_13 import FieldTypeType13
from ..models.field_type_type_14 import FieldTypeType14
from ..models.field_type_type_15 import FieldTypeType15
from ..models.field_type_type_16 import FieldTypeType16
from ..models.field_type_type_17 import FieldTypeType17
from ..models.field_type_type_19 import FieldTypeType19
from ..models.field_type_type_20 import FieldTypeType20
from ..models.field_type_type_21 import FieldTypeType21
from ..models.field_type_type_22 import FieldTypeType22
from ..models.field_type_type_23 import FieldTypeType23
from ..models.field_type_type_24 import FieldTypeType24
from ..models.field_type_type_26 import FieldTypeType26
from ..models.field_type_type_27 import FieldTypeType27
from ..models.field_type_type_28 import FieldTypeType28
from ..models.field_type_type_29 import FieldTypeType29
from ..models.field_type_type_30 import FieldTypeType30
from ..models.field_type_type_32 import FieldTypeType32
from ..models.field_type_type_33 import FieldTypeType33
from ..models.field_type_type_35 import FieldTypeType35
from ..models.field_type_type_36 import FieldTypeType36
from ..models.field_type_type_37 import FieldTypeType37
from ..models.field_type_type_38 import FieldTypeType38
from ..models.field_type_type_39 import FieldTypeType39
from ..models.field_type_type_40 import FieldTypeType40
from ..models.field_type_type_41 import FieldTypeType41
from ..models.field_type_type_42 import FieldTypeType42
from ..models.field_type_type_43 import FieldTypeType43
from ..models.field_type_type_44 import FieldTypeType44
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.field_type_type_9 import FieldTypeType9
    from ..models.field_type_type_10 import FieldTypeType10
    from ..models.field_type_type_18 import FieldTypeType18
    from ..models.field_type_type_25 import FieldTypeType25
    from ..models.field_type_type_31 import FieldTypeType31
    from ..models.field_type_type_34 import FieldTypeType34


T = TypeVar("T", bound="RequiredFieldInfo")


@_attrs_define
class RequiredFieldInfo:
    """Required fields info used while listing the payment_method_data

    Attributes:
        required_field (str): Required field for a payment_method through a payment_method_type
        display_name (str): Display name of the required field in the front-end
        field_type (Union['FieldTypeType10', 'FieldTypeType18', 'FieldTypeType25', 'FieldTypeType31', 'FieldTypeType34',
            'FieldTypeType9', FieldTypeType0, FieldTypeType1, FieldTypeType11, FieldTypeType12, FieldTypeType13,
            FieldTypeType14, FieldTypeType15, FieldTypeType16, FieldTypeType17, FieldTypeType19, FieldTypeType2,
            FieldTypeType20, FieldTypeType21, FieldTypeType22, FieldTypeType23, FieldTypeType24, FieldTypeType26,
            FieldTypeType27, FieldTypeType28, FieldTypeType29, FieldTypeType3, FieldTypeType30, FieldTypeType32,
            FieldTypeType33, FieldTypeType35, FieldTypeType36, FieldTypeType37, FieldTypeType38, FieldTypeType39,
            FieldTypeType4, FieldTypeType40, FieldTypeType41, FieldTypeType42, FieldTypeType43, FieldTypeType44,
            FieldTypeType5, FieldTypeType6, FieldTypeType7, FieldTypeType8]): Possible field type of required fields in
            payment_method_data
        value (Union[None, Unset, str]):
    """

    required_field: str
    display_name: str
    field_type: Union[
        "FieldTypeType10",
        "FieldTypeType18",
        "FieldTypeType25",
        "FieldTypeType31",
        "FieldTypeType34",
        "FieldTypeType9",
        FieldTypeType0,
        FieldTypeType1,
        FieldTypeType11,
        FieldTypeType12,
        FieldTypeType13,
        FieldTypeType14,
        FieldTypeType15,
        FieldTypeType16,
        FieldTypeType17,
        FieldTypeType19,
        FieldTypeType2,
        FieldTypeType20,
        FieldTypeType21,
        FieldTypeType22,
        FieldTypeType23,
        FieldTypeType24,
        FieldTypeType26,
        FieldTypeType27,
        FieldTypeType28,
        FieldTypeType29,
        FieldTypeType3,
        FieldTypeType30,
        FieldTypeType32,
        FieldTypeType33,
        FieldTypeType35,
        FieldTypeType36,
        FieldTypeType37,
        FieldTypeType38,
        FieldTypeType39,
        FieldTypeType4,
        FieldTypeType40,
        FieldTypeType41,
        FieldTypeType42,
        FieldTypeType43,
        FieldTypeType44,
        FieldTypeType5,
        FieldTypeType6,
        FieldTypeType7,
        FieldTypeType8,
    ]
    value: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.field_type_type_9 import FieldTypeType9
        from ..models.field_type_type_10 import FieldTypeType10
        from ..models.field_type_type_18 import FieldTypeType18
        from ..models.field_type_type_25 import FieldTypeType25
        from ..models.field_type_type_31 import FieldTypeType31
        from ..models.field_type_type_34 import FieldTypeType34

        required_field = self.required_field

        display_name = self.display_name

        field_type: Union[dict[str, Any], str]
        if isinstance(self.field_type, FieldTypeType0):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType1):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType2):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType3):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType4):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType5):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType6):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType7):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType8):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType9):
            field_type = self.field_type.to_dict()
        elif isinstance(self.field_type, FieldTypeType10):
            field_type = self.field_type.to_dict()
        elif isinstance(self.field_type, FieldTypeType11):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType12):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType13):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType14):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType15):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType16):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType17):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType18):
            field_type = self.field_type.to_dict()
        elif isinstance(self.field_type, FieldTypeType19):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType20):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType21):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType22):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType23):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType24):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType25):
            field_type = self.field_type.to_dict()
        elif isinstance(self.field_type, FieldTypeType26):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType27):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType28):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType29):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType30):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType31):
            field_type = self.field_type.to_dict()
        elif isinstance(self.field_type, FieldTypeType32):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType33):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType34):
            field_type = self.field_type.to_dict()
        elif isinstance(self.field_type, FieldTypeType35):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType36):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType37):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType38):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType39):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType40):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType41):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType42):
            field_type = self.field_type.value
        elif isinstance(self.field_type, FieldTypeType43):
            field_type = self.field_type.value
        else:
            field_type = self.field_type.value

        value: Union[None, Unset, str]
        if isinstance(self.value, Unset):
            value = UNSET
        else:
            value = self.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "required_field": required_field,
                "display_name": display_name,
                "field_type": field_type,
            }
        )
        if value is not UNSET:
            field_dict["value"] = value

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.field_type_type_9 import FieldTypeType9
        from ..models.field_type_type_10 import FieldTypeType10
        from ..models.field_type_type_18 import FieldTypeType18
        from ..models.field_type_type_25 import FieldTypeType25
        from ..models.field_type_type_31 import FieldTypeType31
        from ..models.field_type_type_34 import FieldTypeType34

        d = dict(src_dict)
        required_field = d.pop("required_field")

        display_name = d.pop("display_name")

        def _parse_field_type(
            data: object,
        ) -> Union[
            "FieldTypeType10",
            "FieldTypeType18",
            "FieldTypeType25",
            "FieldTypeType31",
            "FieldTypeType34",
            "FieldTypeType9",
            FieldTypeType0,
            FieldTypeType1,
            FieldTypeType11,
            FieldTypeType12,
            FieldTypeType13,
            FieldTypeType14,
            FieldTypeType15,
            FieldTypeType16,
            FieldTypeType17,
            FieldTypeType19,
            FieldTypeType2,
            FieldTypeType20,
            FieldTypeType21,
            FieldTypeType22,
            FieldTypeType23,
            FieldTypeType24,
            FieldTypeType26,
            FieldTypeType27,
            FieldTypeType28,
            FieldTypeType29,
            FieldTypeType3,
            FieldTypeType30,
            FieldTypeType32,
            FieldTypeType33,
            FieldTypeType35,
            FieldTypeType36,
            FieldTypeType37,
            FieldTypeType38,
            FieldTypeType39,
            FieldTypeType4,
            FieldTypeType40,
            FieldTypeType41,
            FieldTypeType42,
            FieldTypeType43,
            FieldTypeType44,
            FieldTypeType5,
            FieldTypeType6,
            FieldTypeType7,
            FieldTypeType8,
        ]:
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_0 = FieldTypeType0(data)

                return componentsschemas_field_type_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_1 = FieldTypeType1(data)

                return componentsschemas_field_type_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_2 = FieldTypeType2(data)

                return componentsschemas_field_type_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_3 = FieldTypeType3(data)

                return componentsschemas_field_type_type_3
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_4 = FieldTypeType4(data)

                return componentsschemas_field_type_type_4
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_5 = FieldTypeType5(data)

                return componentsschemas_field_type_type_5
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_6 = FieldTypeType6(data)

                return componentsschemas_field_type_type_6
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_7 = FieldTypeType7(data)

                return componentsschemas_field_type_type_7
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_8 = FieldTypeType8(data)

                return componentsschemas_field_type_type_8
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_field_type_type_9 = FieldTypeType9.from_dict(data)

                return componentsschemas_field_type_type_9
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_field_type_type_10 = FieldTypeType10.from_dict(data)

                return componentsschemas_field_type_type_10
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_11 = FieldTypeType11(data)

                return componentsschemas_field_type_type_11
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_12 = FieldTypeType12(data)

                return componentsschemas_field_type_type_12
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_13 = FieldTypeType13(data)

                return componentsschemas_field_type_type_13
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_14 = FieldTypeType14(data)

                return componentsschemas_field_type_type_14
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_15 = FieldTypeType15(data)

                return componentsschemas_field_type_type_15
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_16 = FieldTypeType16(data)

                return componentsschemas_field_type_type_16
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_17 = FieldTypeType17(data)

                return componentsschemas_field_type_type_17
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_field_type_type_18 = FieldTypeType18.from_dict(data)

                return componentsschemas_field_type_type_18
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_19 = FieldTypeType19(data)

                return componentsschemas_field_type_type_19
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_20 = FieldTypeType20(data)

                return componentsschemas_field_type_type_20
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_21 = FieldTypeType21(data)

                return componentsschemas_field_type_type_21
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_22 = FieldTypeType22(data)

                return componentsschemas_field_type_type_22
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_23 = FieldTypeType23(data)

                return componentsschemas_field_type_type_23
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_24 = FieldTypeType24(data)

                return componentsschemas_field_type_type_24
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_field_type_type_25 = FieldTypeType25.from_dict(data)

                return componentsschemas_field_type_type_25
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_26 = FieldTypeType26(data)

                return componentsschemas_field_type_type_26
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_27 = FieldTypeType27(data)

                return componentsschemas_field_type_type_27
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_28 = FieldTypeType28(data)

                return componentsschemas_field_type_type_28
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_29 = FieldTypeType29(data)

                return componentsschemas_field_type_type_29
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_30 = FieldTypeType30(data)

                return componentsschemas_field_type_type_30
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_field_type_type_31 = FieldTypeType31.from_dict(data)

                return componentsschemas_field_type_type_31
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_32 = FieldTypeType32(data)

                return componentsschemas_field_type_type_32
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_33 = FieldTypeType33(data)

                return componentsschemas_field_type_type_33
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_field_type_type_34 = FieldTypeType34.from_dict(data)

                return componentsschemas_field_type_type_34
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_35 = FieldTypeType35(data)

                return componentsschemas_field_type_type_35
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_36 = FieldTypeType36(data)

                return componentsschemas_field_type_type_36
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_37 = FieldTypeType37(data)

                return componentsschemas_field_type_type_37
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_38 = FieldTypeType38(data)

                return componentsschemas_field_type_type_38
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_39 = FieldTypeType39(data)

                return componentsschemas_field_type_type_39
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_40 = FieldTypeType40(data)

                return componentsschemas_field_type_type_40
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_41 = FieldTypeType41(data)

                return componentsschemas_field_type_type_41
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_42 = FieldTypeType42(data)

                return componentsschemas_field_type_type_42
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, str):
                    raise TypeError()
                componentsschemas_field_type_type_43 = FieldTypeType43(data)

                return componentsschemas_field_type_type_43
            except:  # noqa: E722
                pass
            if not isinstance(data, str):
                raise TypeError()
            componentsschemas_field_type_type_44 = FieldTypeType44(data)

            return componentsschemas_field_type_type_44

        field_type = _parse_field_type(d.pop("field_type"))

        def _parse_value(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        value = _parse_value(d.pop("value", UNSET))

        required_field_info = cls(
            required_field=required_field,
            display_name=display_name,
            field_type=field_type,
            value=value,
        )

        required_field_info.additional_properties = d
        return required_field_info

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
