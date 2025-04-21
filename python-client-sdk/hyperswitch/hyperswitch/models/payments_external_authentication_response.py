from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.transaction_status import TransactionStatus
from ..types import UNSET, Unset

T = TypeVar("T", bound="PaymentsExternalAuthenticationResponse")


@_attrs_define
class PaymentsExternalAuthenticationResponse:
    """
    Attributes:
        trans_status (TransactionStatus): Indicates the transaction status
        three_ds_requestor_url (str): Three DS Requestor URL
        acs_url (Union[None, Unset, str]): Access Server URL to be used for challenge submission
        challenge_request (Union[None, Unset, str]): Challenge request which should be sent to acs_url
        acs_reference_number (Union[None, Unset, str]): Unique identifier assigned by the EMVCo(Europay, Mastercard and
            Visa)
        acs_trans_id (Union[None, Unset, str]): Unique identifier assigned by the ACS to identify a single transaction
        three_dsserver_trans_id (Union[None, Unset, str]): Unique identifier assigned by the 3DS Server to identify a
            single transaction
        acs_signed_content (Union[None, Unset, str]): Contains the JWS object created by the ACS for the
            ARes(Authentication Response) message
        three_ds_requestor_app_url (Union[None, Unset, str]): Merchant app declaring their URL within the CReq message
            so that the Authentication app can call the Merchant app after OOB authentication has occurred
    """

    trans_status: TransactionStatus
    three_ds_requestor_url: str
    acs_url: Union[None, Unset, str] = UNSET
    challenge_request: Union[None, Unset, str] = UNSET
    acs_reference_number: Union[None, Unset, str] = UNSET
    acs_trans_id: Union[None, Unset, str] = UNSET
    three_dsserver_trans_id: Union[None, Unset, str] = UNSET
    acs_signed_content: Union[None, Unset, str] = UNSET
    three_ds_requestor_app_url: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        trans_status = self.trans_status.value

        three_ds_requestor_url = self.three_ds_requestor_url

        acs_url: Union[None, Unset, str]
        if isinstance(self.acs_url, Unset):
            acs_url = UNSET
        else:
            acs_url = self.acs_url

        challenge_request: Union[None, Unset, str]
        if isinstance(self.challenge_request, Unset):
            challenge_request = UNSET
        else:
            challenge_request = self.challenge_request

        acs_reference_number: Union[None, Unset, str]
        if isinstance(self.acs_reference_number, Unset):
            acs_reference_number = UNSET
        else:
            acs_reference_number = self.acs_reference_number

        acs_trans_id: Union[None, Unset, str]
        if isinstance(self.acs_trans_id, Unset):
            acs_trans_id = UNSET
        else:
            acs_trans_id = self.acs_trans_id

        three_dsserver_trans_id: Union[None, Unset, str]
        if isinstance(self.three_dsserver_trans_id, Unset):
            three_dsserver_trans_id = UNSET
        else:
            three_dsserver_trans_id = self.three_dsserver_trans_id

        acs_signed_content: Union[None, Unset, str]
        if isinstance(self.acs_signed_content, Unset):
            acs_signed_content = UNSET
        else:
            acs_signed_content = self.acs_signed_content

        three_ds_requestor_app_url: Union[None, Unset, str]
        if isinstance(self.three_ds_requestor_app_url, Unset):
            three_ds_requestor_app_url = UNSET
        else:
            three_ds_requestor_app_url = self.three_ds_requestor_app_url

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "trans_status": trans_status,
                "three_ds_requestor_url": three_ds_requestor_url,
            }
        )
        if acs_url is not UNSET:
            field_dict["acs_url"] = acs_url
        if challenge_request is not UNSET:
            field_dict["challenge_request"] = challenge_request
        if acs_reference_number is not UNSET:
            field_dict["acs_reference_number"] = acs_reference_number
        if acs_trans_id is not UNSET:
            field_dict["acs_trans_id"] = acs_trans_id
        if three_dsserver_trans_id is not UNSET:
            field_dict["three_dsserver_trans_id"] = three_dsserver_trans_id
        if acs_signed_content is not UNSET:
            field_dict["acs_signed_content"] = acs_signed_content
        if three_ds_requestor_app_url is not UNSET:
            field_dict["three_ds_requestor_app_url"] = three_ds_requestor_app_url

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        trans_status = TransactionStatus(d.pop("trans_status"))

        three_ds_requestor_url = d.pop("three_ds_requestor_url")

        def _parse_acs_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        acs_url = _parse_acs_url(d.pop("acs_url", UNSET))

        def _parse_challenge_request(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        challenge_request = _parse_challenge_request(d.pop("challenge_request", UNSET))

        def _parse_acs_reference_number(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        acs_reference_number = _parse_acs_reference_number(d.pop("acs_reference_number", UNSET))

        def _parse_acs_trans_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        acs_trans_id = _parse_acs_trans_id(d.pop("acs_trans_id", UNSET))

        def _parse_three_dsserver_trans_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        three_dsserver_trans_id = _parse_three_dsserver_trans_id(d.pop("three_dsserver_trans_id", UNSET))

        def _parse_acs_signed_content(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        acs_signed_content = _parse_acs_signed_content(d.pop("acs_signed_content", UNSET))

        def _parse_three_ds_requestor_app_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        three_ds_requestor_app_url = _parse_three_ds_requestor_app_url(d.pop("three_ds_requestor_app_url", UNSET))

        payments_external_authentication_response = cls(
            trans_status=trans_status,
            three_ds_requestor_url=three_ds_requestor_url,
            acs_url=acs_url,
            challenge_request=challenge_request,
            acs_reference_number=acs_reference_number,
            acs_trans_id=acs_trans_id,
            three_dsserver_trans_id=three_dsserver_trans_id,
            acs_signed_content=acs_signed_content,
            three_ds_requestor_app_url=three_ds_requestor_app_url,
        )

        payments_external_authentication_response.additional_properties = d
        return payments_external_authentication_response

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
