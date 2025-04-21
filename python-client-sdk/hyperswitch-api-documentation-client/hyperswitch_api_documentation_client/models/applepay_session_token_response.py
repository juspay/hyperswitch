from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.apple_pay_payment_request import ApplePayPaymentRequest
    from ..models.apple_pay_session_response_type_2_type_0 import ApplePaySessionResponseType2Type0
    from ..models.no_third_party_sdk_session_response import NoThirdPartySdkSessionResponse
    from ..models.sdk_next_action import SdkNextAction
    from ..models.third_party_sdk_session_response import ThirdPartySdkSessionResponse


T = TypeVar("T", bound="ApplepaySessionTokenResponse")


@_attrs_define
class ApplepaySessionTokenResponse:
    """
    Attributes:
        connector (str): The session token is w.r.t this connector
        delayed_session_token (bool): Identifier for the delayed session response
        sdk_next_action (SdkNextAction):
        session_token_data (Union['ApplePaySessionResponseType2Type0', 'NoThirdPartySdkSessionResponse',
            'ThirdPartySdkSessionResponse', None, Unset]):
        payment_request_data (Union['ApplePayPaymentRequest', None, Unset]):
        connector_reference_id (Union[None, Unset, str]): The connector transaction id
        connector_sdk_public_key (Union[None, Unset, str]): The public key id is to invoke third party sdk
        connector_merchant_id (Union[None, Unset, str]): The connector merchant id
    """

    connector: str
    delayed_session_token: bool
    sdk_next_action: "SdkNextAction"
    session_token_data: Union[
        "ApplePaySessionResponseType2Type0",
        "NoThirdPartySdkSessionResponse",
        "ThirdPartySdkSessionResponse",
        None,
        Unset,
    ] = UNSET
    payment_request_data: Union["ApplePayPaymentRequest", None, Unset] = UNSET
    connector_reference_id: Union[None, Unset, str] = UNSET
    connector_sdk_public_key: Union[None, Unset, str] = UNSET
    connector_merchant_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.apple_pay_payment_request import ApplePayPaymentRequest
        from ..models.apple_pay_session_response_type_2_type_0 import ApplePaySessionResponseType2Type0
        from ..models.no_third_party_sdk_session_response import NoThirdPartySdkSessionResponse
        from ..models.third_party_sdk_session_response import ThirdPartySdkSessionResponse

        connector = self.connector

        delayed_session_token = self.delayed_session_token

        sdk_next_action = self.sdk_next_action.to_dict()

        session_token_data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.session_token_data, Unset):
            session_token_data = UNSET
        elif isinstance(self.session_token_data, ThirdPartySdkSessionResponse):
            session_token_data = self.session_token_data.to_dict()
        elif isinstance(self.session_token_data, NoThirdPartySdkSessionResponse):
            session_token_data = self.session_token_data.to_dict()
        elif isinstance(self.session_token_data, ApplePaySessionResponseType2Type0):
            session_token_data = self.session_token_data.to_dict()
        else:
            session_token_data = self.session_token_data

        payment_request_data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payment_request_data, Unset):
            payment_request_data = UNSET
        elif isinstance(self.payment_request_data, ApplePayPaymentRequest):
            payment_request_data = self.payment_request_data.to_dict()
        else:
            payment_request_data = self.payment_request_data

        connector_reference_id: Union[None, Unset, str]
        if isinstance(self.connector_reference_id, Unset):
            connector_reference_id = UNSET
        else:
            connector_reference_id = self.connector_reference_id

        connector_sdk_public_key: Union[None, Unset, str]
        if isinstance(self.connector_sdk_public_key, Unset):
            connector_sdk_public_key = UNSET
        else:
            connector_sdk_public_key = self.connector_sdk_public_key

        connector_merchant_id: Union[None, Unset, str]
        if isinstance(self.connector_merchant_id, Unset):
            connector_merchant_id = UNSET
        else:
            connector_merchant_id = self.connector_merchant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector": connector,
                "delayed_session_token": delayed_session_token,
                "sdk_next_action": sdk_next_action,
            }
        )
        if session_token_data is not UNSET:
            field_dict["session_token_data"] = session_token_data
        if payment_request_data is not UNSET:
            field_dict["payment_request_data"] = payment_request_data
        if connector_reference_id is not UNSET:
            field_dict["connector_reference_id"] = connector_reference_id
        if connector_sdk_public_key is not UNSET:
            field_dict["connector_sdk_public_key"] = connector_sdk_public_key
        if connector_merchant_id is not UNSET:
            field_dict["connector_merchant_id"] = connector_merchant_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.apple_pay_payment_request import ApplePayPaymentRequest
        from ..models.apple_pay_session_response_type_2_type_0 import ApplePaySessionResponseType2Type0
        from ..models.no_third_party_sdk_session_response import NoThirdPartySdkSessionResponse
        from ..models.sdk_next_action import SdkNextAction
        from ..models.third_party_sdk_session_response import ThirdPartySdkSessionResponse

        d = dict(src_dict)
        connector = d.pop("connector")

        delayed_session_token = d.pop("delayed_session_token")

        sdk_next_action = SdkNextAction.from_dict(d.pop("sdk_next_action"))

        def _parse_session_token_data(
            data: object,
        ) -> Union[
            "ApplePaySessionResponseType2Type0",
            "NoThirdPartySdkSessionResponse",
            "ThirdPartySdkSessionResponse",
            None,
            Unset,
        ]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_apple_pay_session_response_type_0 = ThirdPartySdkSessionResponse.from_dict(data)

                return componentsschemas_apple_pay_session_response_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_apple_pay_session_response_type_1 = NoThirdPartySdkSessionResponse.from_dict(data)

                return componentsschemas_apple_pay_session_response_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_apple_pay_session_response_type_2_type_0 = (
                    ApplePaySessionResponseType2Type0.from_dict(data)
                )

                return componentsschemas_apple_pay_session_response_type_2_type_0
            except:  # noqa: E722
                pass
            return cast(
                Union[
                    "ApplePaySessionResponseType2Type0",
                    "NoThirdPartySdkSessionResponse",
                    "ThirdPartySdkSessionResponse",
                    None,
                    Unset,
                ],
                data,
            )

        session_token_data = _parse_session_token_data(d.pop("session_token_data", UNSET))

        def _parse_payment_request_data(data: object) -> Union["ApplePayPaymentRequest", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                payment_request_data_type_1 = ApplePayPaymentRequest.from_dict(data)

                return payment_request_data_type_1
            except:  # noqa: E722
                pass
            return cast(Union["ApplePayPaymentRequest", None, Unset], data)

        payment_request_data = _parse_payment_request_data(d.pop("payment_request_data", UNSET))

        def _parse_connector_reference_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_reference_id = _parse_connector_reference_id(d.pop("connector_reference_id", UNSET))

        def _parse_connector_sdk_public_key(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_sdk_public_key = _parse_connector_sdk_public_key(d.pop("connector_sdk_public_key", UNSET))

        def _parse_connector_merchant_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_merchant_id = _parse_connector_merchant_id(d.pop("connector_merchant_id", UNSET))

        applepay_session_token_response = cls(
            connector=connector,
            delayed_session_token=delayed_session_token,
            sdk_next_action=sdk_next_action,
            session_token_data=session_token_data,
            payment_request_data=payment_request_data,
            connector_reference_id=connector_reference_id,
            connector_sdk_public_key=connector_sdk_public_key,
            connector_merchant_id=connector_merchant_id,
        )

        applepay_session_token_response.additional_properties = d
        return applepay_session_token_response

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
