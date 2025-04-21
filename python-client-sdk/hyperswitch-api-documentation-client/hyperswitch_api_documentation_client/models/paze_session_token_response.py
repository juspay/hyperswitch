from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.currency import Currency
from ..types import UNSET, Unset

T = TypeVar("T", bound="PazeSessionTokenResponse")


@_attrs_define
class PazeSessionTokenResponse:
    """
    Attributes:
        client_id (str): Paze Client ID
        client_name (str): Client Name to be displayed on the Paze screen
        client_profile_id (str): Paze Client Profile ID
        transaction_currency_code (Currency): The three letter ISO currency code in uppercase. Eg: 'USD' for the United
            States Dollar.
        transaction_amount (str): The transaction amount Example: 38.02.
        email_address (Union[None, Unset, str]): Email Address Example: johntest@test.com.
    """

    client_id: str
    client_name: str
    client_profile_id: str
    transaction_currency_code: Currency
    transaction_amount: str
    email_address: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        client_id = self.client_id

        client_name = self.client_name

        client_profile_id = self.client_profile_id

        transaction_currency_code = self.transaction_currency_code.value

        transaction_amount = self.transaction_amount

        email_address: Union[None, Unset, str]
        if isinstance(self.email_address, Unset):
            email_address = UNSET
        else:
            email_address = self.email_address

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "client_id": client_id,
                "client_name": client_name,
                "client_profile_id": client_profile_id,
                "transaction_currency_code": transaction_currency_code,
                "transaction_amount": transaction_amount,
            }
        )
        if email_address is not UNSET:
            field_dict["email_address"] = email_address

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        client_id = d.pop("client_id")

        client_name = d.pop("client_name")

        client_profile_id = d.pop("client_profile_id")

        transaction_currency_code = Currency(d.pop("transaction_currency_code"))

        transaction_amount = d.pop("transaction_amount")

        def _parse_email_address(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        email_address = _parse_email_address(d.pop("email_address", UNSET))

        paze_session_token_response = cls(
            client_id=client_id,
            client_name=client_name,
            client_profile_id=client_profile_id,
            transaction_currency_code=transaction_currency_code,
            transaction_amount=transaction_amount,
            email_address=email_address,
        )

        paze_session_token_response.additional_properties = d
        return paze_session_token_response

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
