from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="BankTransferDataType11Pix")


@_attrs_define
class BankTransferDataType11Pix:
    """
    Attributes:
        pix_key (Union[None, Unset, str]): Unique key for pix transfer Example: a1f4102e-a446-4a57-bcce-6fa48899c1d1.
        cpf (Union[None, Unset, str]): CPF is a Brazilian tax identification number Example: 10599054689.
        cnpj (Union[None, Unset, str]): CNPJ is a Brazilian company tax identification number Example: 74469027417312.
    """

    pix_key: Union[None, Unset, str] = UNSET
    cpf: Union[None, Unset, str] = UNSET
    cnpj: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        pix_key: Union[None, Unset, str]
        if isinstance(self.pix_key, Unset):
            pix_key = UNSET
        else:
            pix_key = self.pix_key

        cpf: Union[None, Unset, str]
        if isinstance(self.cpf, Unset):
            cpf = UNSET
        else:
            cpf = self.cpf

        cnpj: Union[None, Unset, str]
        if isinstance(self.cnpj, Unset):
            cnpj = UNSET
        else:
            cnpj = self.cnpj

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if pix_key is not UNSET:
            field_dict["pix_key"] = pix_key
        if cpf is not UNSET:
            field_dict["cpf"] = cpf
        if cnpj is not UNSET:
            field_dict["cnpj"] = cnpj

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_pix_key(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        pix_key = _parse_pix_key(d.pop("pix_key", UNSET))

        def _parse_cpf(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        cpf = _parse_cpf(d.pop("cpf", UNSET))

        def _parse_cnpj(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        cnpj = _parse_cnpj(d.pop("cnpj", UNSET))

        bank_transfer_data_type_11_pix = cls(
            pix_key=pix_key,
            cpf=cpf,
            cnpj=cnpj,
        )

        bank_transfer_data_type_11_pix.additional_properties = d
        return bank_transfer_data_type_11_pix

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
