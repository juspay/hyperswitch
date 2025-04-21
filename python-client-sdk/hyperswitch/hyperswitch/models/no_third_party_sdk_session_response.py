from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="NoThirdPartySdkSessionResponse")


@_attrs_define
class NoThirdPartySdkSessionResponse:
    """
    Attributes:
        epoch_timestamp (int): Timestamp at which session is requested
        expires_at (int): Timestamp at which session expires
        merchant_session_identifier (str): The identifier for the merchant session
        nonce (str): Apple pay generated unique ID (UUID) value
        merchant_identifier (str): The identifier for the merchant
        domain_name (str): The domain name of the merchant which is registered in Apple Pay
        display_name (str): The name to be displayed on Apple Pay button
        signature (str): A string which represents the properties of a payment
        operational_analytics_identifier (str): The identifier for the operational analytics
        retries (int): The number of retries to get the session response
        psp_id (str): The identifier for the connector transaction
    """

    epoch_timestamp: int
    expires_at: int
    merchant_session_identifier: str
    nonce: str
    merchant_identifier: str
    domain_name: str
    display_name: str
    signature: str
    operational_analytics_identifier: str
    retries: int
    psp_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        epoch_timestamp = self.epoch_timestamp

        expires_at = self.expires_at

        merchant_session_identifier = self.merchant_session_identifier

        nonce = self.nonce

        merchant_identifier = self.merchant_identifier

        domain_name = self.domain_name

        display_name = self.display_name

        signature = self.signature

        operational_analytics_identifier = self.operational_analytics_identifier

        retries = self.retries

        psp_id = self.psp_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "epoch_timestamp": epoch_timestamp,
                "expires_at": expires_at,
                "merchant_session_identifier": merchant_session_identifier,
                "nonce": nonce,
                "merchant_identifier": merchant_identifier,
                "domain_name": domain_name,
                "display_name": display_name,
                "signature": signature,
                "operational_analytics_identifier": operational_analytics_identifier,
                "retries": retries,
                "psp_id": psp_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        epoch_timestamp = d.pop("epoch_timestamp")

        expires_at = d.pop("expires_at")

        merchant_session_identifier = d.pop("merchant_session_identifier")

        nonce = d.pop("nonce")

        merchant_identifier = d.pop("merchant_identifier")

        domain_name = d.pop("domain_name")

        display_name = d.pop("display_name")

        signature = d.pop("signature")

        operational_analytics_identifier = d.pop("operational_analytics_identifier")

        retries = d.pop("retries")

        psp_id = d.pop("psp_id")

        no_third_party_sdk_session_response = cls(
            epoch_timestamp=epoch_timestamp,
            expires_at=expires_at,
            merchant_session_identifier=merchant_session_identifier,
            nonce=nonce,
            merchant_identifier=merchant_identifier,
            domain_name=domain_name,
            display_name=display_name,
            signature=signature,
            operational_analytics_identifier=operational_analytics_identifier,
            retries=retries,
            psp_id=psp_id,
        )

        no_third_party_sdk_session_response.additional_properties = d
        return no_third_party_sdk_session_response

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
