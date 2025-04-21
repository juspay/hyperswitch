from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.merchant_product_type import MerchantProductType
from ..models.recon_status import ReconStatus
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.business_collect_link_config import BusinessCollectLinkConfig
    from ..models.merchant_account_response_metadata_type_0 import MerchantAccountResponseMetadataType0
    from ..models.merchant_details import MerchantDetails
    from ..models.primary_business_details import PrimaryBusinessDetails
    from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
    from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
    from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
    from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3
    from ..models.webhook_details import WebhookDetails


T = TypeVar("T", bound="MerchantAccountResponse")


@_attrs_define
class MerchantAccountResponse:
    """
    Attributes:
        merchant_id (str): The identifier for the Merchant Account Example: y3oqhf46pyzuxjbcn2giaqnb44.
        enable_payment_response_hash (bool): A boolean value to indicate if payment response hash needs to be enabled
            Default: False. Example: True.
        redirect_to_merchant_with_http_post (bool): A boolean value to indicate if redirect to merchant with http post
            needs to be enabled Default: False. Example: True.
        primary_business_details (list['PrimaryBusinessDetails']): Details about the primary business unit of the
            merchant account
        organization_id (str): The organization id merchant is associated with Example: org_q98uSGAYbjEwqs0mJwnz.
        is_recon_enabled (bool): A boolean value to indicate if the merchant has recon service is enabled or not, by
            default value is false
        recon_status (ReconStatus):
        merchant_name (Union[None, Unset, str]): Name of the Merchant Account Example: NewAge Retailer.
        return_url (Union[None, Unset, str]): The URL to redirect after completion of the payment Example:
            https://www.example.com/success.
        payment_response_hash_key (Union[None, Unset, str]): Refers to the hash key used for calculating the signature
            for webhooks and redirect response. If the value is not provided, a value is automatically generated. Example:
            xkkdf909012sdjki2dkh5sdf.
        merchant_details (Union['MerchantDetails', None, Unset]):
        webhook_details (Union['WebhookDetails', None, Unset]):
        payout_routing_algorithm (Union['RoutingAlgorithmType0', 'RoutingAlgorithmType1', 'RoutingAlgorithmType2',
            'RoutingAlgorithmType3', None, Unset]):
        sub_merchants_enabled (Union[None, Unset, bool]): A boolean value to indicate if the merchant is a sub-merchant
            under a master or a parent merchant. By default, its value is false. Default: False.
        parent_merchant_id (Union[None, Unset, str]): Refers to the Parent Merchant ID if the merchant being created is
            a sub-merchant Example: xkkdf909012sdjki2dkh5sdf.
        publishable_key (Union[None, Unset, str]): API key that will be used for server side API access Example:
            AH3423bkjbkjdsfbkj.
        metadata (Union['MerchantAccountResponseMetadataType0', None, Unset]): Metadata is useful for storing
            additional, unstructured information on an object.
        locker_id (Union[None, Unset, str]): An identifier for the vault used to store payment method information.
            Example: locker_abc123.
        frm_routing_algorithm (Union['RoutingAlgorithmType0', 'RoutingAlgorithmType1', 'RoutingAlgorithmType2',
            'RoutingAlgorithmType3', None, Unset]):
        default_profile (Union[None, Unset, str]): The default profile that must be used for creating merchant accounts
            and payments
        pm_collect_link_config (Union['BusinessCollectLinkConfig', None, Unset]):
        product_type (Union[MerchantProductType, None, Unset]):
    """

    merchant_id: str
    primary_business_details: list["PrimaryBusinessDetails"]
    organization_id: str
    is_recon_enabled: bool
    recon_status: ReconStatus
    enable_payment_response_hash: bool = False
    redirect_to_merchant_with_http_post: bool = False
    merchant_name: Union[None, Unset, str] = UNSET
    return_url: Union[None, Unset, str] = UNSET
    payment_response_hash_key: Union[None, Unset, str] = UNSET
    merchant_details: Union["MerchantDetails", None, Unset] = UNSET
    webhook_details: Union["WebhookDetails", None, Unset] = UNSET
    payout_routing_algorithm: Union[
        "RoutingAlgorithmType0", "RoutingAlgorithmType1", "RoutingAlgorithmType2", "RoutingAlgorithmType3", None, Unset
    ] = UNSET
    sub_merchants_enabled: Union[None, Unset, bool] = False
    parent_merchant_id: Union[None, Unset, str] = UNSET
    publishable_key: Union[None, Unset, str] = UNSET
    metadata: Union["MerchantAccountResponseMetadataType0", None, Unset] = UNSET
    locker_id: Union[None, Unset, str] = UNSET
    frm_routing_algorithm: Union[
        "RoutingAlgorithmType0", "RoutingAlgorithmType1", "RoutingAlgorithmType2", "RoutingAlgorithmType3", None, Unset
    ] = UNSET
    default_profile: Union[None, Unset, str] = UNSET
    pm_collect_link_config: Union["BusinessCollectLinkConfig", None, Unset] = UNSET
    product_type: Union[MerchantProductType, None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.business_collect_link_config import BusinessCollectLinkConfig
        from ..models.merchant_account_response_metadata_type_0 import MerchantAccountResponseMetadataType0
        from ..models.merchant_details import MerchantDetails
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
        from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3
        from ..models.webhook_details import WebhookDetails

        merchant_id = self.merchant_id

        enable_payment_response_hash = self.enable_payment_response_hash

        redirect_to_merchant_with_http_post = self.redirect_to_merchant_with_http_post

        primary_business_details = []
        for primary_business_details_item_data in self.primary_business_details:
            primary_business_details_item = primary_business_details_item_data.to_dict()
            primary_business_details.append(primary_business_details_item)

        organization_id = self.organization_id

        is_recon_enabled = self.is_recon_enabled

        recon_status = self.recon_status.value

        merchant_name: Union[None, Unset, str]
        if isinstance(self.merchant_name, Unset):
            merchant_name = UNSET
        else:
            merchant_name = self.merchant_name

        return_url: Union[None, Unset, str]
        if isinstance(self.return_url, Unset):
            return_url = UNSET
        else:
            return_url = self.return_url

        payment_response_hash_key: Union[None, Unset, str]
        if isinstance(self.payment_response_hash_key, Unset):
            payment_response_hash_key = UNSET
        else:
            payment_response_hash_key = self.payment_response_hash_key

        merchant_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.merchant_details, Unset):
            merchant_details = UNSET
        elif isinstance(self.merchant_details, MerchantDetails):
            merchant_details = self.merchant_details.to_dict()
        else:
            merchant_details = self.merchant_details

        webhook_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.webhook_details, Unset):
            webhook_details = UNSET
        elif isinstance(self.webhook_details, WebhookDetails):
            webhook_details = self.webhook_details.to_dict()
        else:
            webhook_details = self.webhook_details

        payout_routing_algorithm: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payout_routing_algorithm, Unset):
            payout_routing_algorithm = UNSET
        elif isinstance(self.payout_routing_algorithm, RoutingAlgorithmType0):
            payout_routing_algorithm = self.payout_routing_algorithm.to_dict()
        elif isinstance(self.payout_routing_algorithm, RoutingAlgorithmType1):
            payout_routing_algorithm = self.payout_routing_algorithm.to_dict()
        elif isinstance(self.payout_routing_algorithm, RoutingAlgorithmType2):
            payout_routing_algorithm = self.payout_routing_algorithm.to_dict()
        elif isinstance(self.payout_routing_algorithm, RoutingAlgorithmType3):
            payout_routing_algorithm = self.payout_routing_algorithm.to_dict()
        else:
            payout_routing_algorithm = self.payout_routing_algorithm

        sub_merchants_enabled: Union[None, Unset, bool]
        if isinstance(self.sub_merchants_enabled, Unset):
            sub_merchants_enabled = UNSET
        else:
            sub_merchants_enabled = self.sub_merchants_enabled

        parent_merchant_id: Union[None, Unset, str]
        if isinstance(self.parent_merchant_id, Unset):
            parent_merchant_id = UNSET
        else:
            parent_merchant_id = self.parent_merchant_id

        publishable_key: Union[None, Unset, str]
        if isinstance(self.publishable_key, Unset):
            publishable_key = UNSET
        else:
            publishable_key = self.publishable_key

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, MerchantAccountResponseMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        locker_id: Union[None, Unset, str]
        if isinstance(self.locker_id, Unset):
            locker_id = UNSET
        else:
            locker_id = self.locker_id

        frm_routing_algorithm: Union[None, Unset, dict[str, Any]]
        if isinstance(self.frm_routing_algorithm, Unset):
            frm_routing_algorithm = UNSET
        elif isinstance(self.frm_routing_algorithm, RoutingAlgorithmType0):
            frm_routing_algorithm = self.frm_routing_algorithm.to_dict()
        elif isinstance(self.frm_routing_algorithm, RoutingAlgorithmType1):
            frm_routing_algorithm = self.frm_routing_algorithm.to_dict()
        elif isinstance(self.frm_routing_algorithm, RoutingAlgorithmType2):
            frm_routing_algorithm = self.frm_routing_algorithm.to_dict()
        elif isinstance(self.frm_routing_algorithm, RoutingAlgorithmType3):
            frm_routing_algorithm = self.frm_routing_algorithm.to_dict()
        else:
            frm_routing_algorithm = self.frm_routing_algorithm

        default_profile: Union[None, Unset, str]
        if isinstance(self.default_profile, Unset):
            default_profile = UNSET
        else:
            default_profile = self.default_profile

        pm_collect_link_config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.pm_collect_link_config, Unset):
            pm_collect_link_config = UNSET
        elif isinstance(self.pm_collect_link_config, BusinessCollectLinkConfig):
            pm_collect_link_config = self.pm_collect_link_config.to_dict()
        else:
            pm_collect_link_config = self.pm_collect_link_config

        product_type: Union[None, Unset, str]
        if isinstance(self.product_type, Unset):
            product_type = UNSET
        elif isinstance(self.product_type, MerchantProductType):
            product_type = self.product_type.value
        else:
            product_type = self.product_type

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "merchant_id": merchant_id,
                "enable_payment_response_hash": enable_payment_response_hash,
                "redirect_to_merchant_with_http_post": redirect_to_merchant_with_http_post,
                "primary_business_details": primary_business_details,
                "organization_id": organization_id,
                "is_recon_enabled": is_recon_enabled,
                "recon_status": recon_status,
            }
        )
        if merchant_name is not UNSET:
            field_dict["merchant_name"] = merchant_name
        if return_url is not UNSET:
            field_dict["return_url"] = return_url
        if payment_response_hash_key is not UNSET:
            field_dict["payment_response_hash_key"] = payment_response_hash_key
        if merchant_details is not UNSET:
            field_dict["merchant_details"] = merchant_details
        if webhook_details is not UNSET:
            field_dict["webhook_details"] = webhook_details
        if payout_routing_algorithm is not UNSET:
            field_dict["payout_routing_algorithm"] = payout_routing_algorithm
        if sub_merchants_enabled is not UNSET:
            field_dict["sub_merchants_enabled"] = sub_merchants_enabled
        if parent_merchant_id is not UNSET:
            field_dict["parent_merchant_id"] = parent_merchant_id
        if publishable_key is not UNSET:
            field_dict["publishable_key"] = publishable_key
        if metadata is not UNSET:
            field_dict["metadata"] = metadata
        if locker_id is not UNSET:
            field_dict["locker_id"] = locker_id
        if frm_routing_algorithm is not UNSET:
            field_dict["frm_routing_algorithm"] = frm_routing_algorithm
        if default_profile is not UNSET:
            field_dict["default_profile"] = default_profile
        if pm_collect_link_config is not UNSET:
            field_dict["pm_collect_link_config"] = pm_collect_link_config
        if product_type is not UNSET:
            field_dict["product_type"] = product_type

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.business_collect_link_config import BusinessCollectLinkConfig
        from ..models.merchant_account_response_metadata_type_0 import MerchantAccountResponseMetadataType0
        from ..models.merchant_details import MerchantDetails
        from ..models.primary_business_details import PrimaryBusinessDetails
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
        from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3
        from ..models.webhook_details import WebhookDetails

        d = dict(src_dict)
        merchant_id = d.pop("merchant_id")

        enable_payment_response_hash = d.pop("enable_payment_response_hash")

        redirect_to_merchant_with_http_post = d.pop("redirect_to_merchant_with_http_post")

        primary_business_details = []
        _primary_business_details = d.pop("primary_business_details")
        for primary_business_details_item_data in _primary_business_details:
            primary_business_details_item = PrimaryBusinessDetails.from_dict(primary_business_details_item_data)

            primary_business_details.append(primary_business_details_item)

        organization_id = d.pop("organization_id")

        is_recon_enabled = d.pop("is_recon_enabled")

        recon_status = ReconStatus(d.pop("recon_status"))

        def _parse_merchant_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_name = _parse_merchant_name(d.pop("merchant_name", UNSET))

        def _parse_return_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        return_url = _parse_return_url(d.pop("return_url", UNSET))

        def _parse_payment_response_hash_key(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_response_hash_key = _parse_payment_response_hash_key(d.pop("payment_response_hash_key", UNSET))

        def _parse_merchant_details(data: object) -> Union["MerchantDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                merchant_details_type_1 = MerchantDetails.from_dict(data)

                return merchant_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantDetails", None, Unset], data)

        merchant_details = _parse_merchant_details(d.pop("merchant_details", UNSET))

        def _parse_webhook_details(data: object) -> Union["WebhookDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                webhook_details_type_1 = WebhookDetails.from_dict(data)

                return webhook_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["WebhookDetails", None, Unset], data)

        webhook_details = _parse_webhook_details(d.pop("webhook_details", UNSET))

        def _parse_payout_routing_algorithm(
            data: object,
        ) -> Union[
            "RoutingAlgorithmType0",
            "RoutingAlgorithmType1",
            "RoutingAlgorithmType2",
            "RoutingAlgorithmType3",
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
                componentsschemas_routing_algorithm_type_0 = RoutingAlgorithmType0.from_dict(data)

                return componentsschemas_routing_algorithm_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_1 = RoutingAlgorithmType1.from_dict(data)

                return componentsschemas_routing_algorithm_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_2 = RoutingAlgorithmType2.from_dict(data)

                return componentsschemas_routing_algorithm_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_3 = RoutingAlgorithmType3.from_dict(data)

                return componentsschemas_routing_algorithm_type_3
            except:  # noqa: E722
                pass
            return cast(
                Union[
                    "RoutingAlgorithmType0",
                    "RoutingAlgorithmType1",
                    "RoutingAlgorithmType2",
                    "RoutingAlgorithmType3",
                    None,
                    Unset,
                ],
                data,
            )

        payout_routing_algorithm = _parse_payout_routing_algorithm(d.pop("payout_routing_algorithm", UNSET))

        def _parse_sub_merchants_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        sub_merchants_enabled = _parse_sub_merchants_enabled(d.pop("sub_merchants_enabled", UNSET))

        def _parse_parent_merchant_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        parent_merchant_id = _parse_parent_merchant_id(d.pop("parent_merchant_id", UNSET))

        def _parse_publishable_key(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        publishable_key = _parse_publishable_key(d.pop("publishable_key", UNSET))

        def _parse_metadata(data: object) -> Union["MerchantAccountResponseMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = MerchantAccountResponseMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantAccountResponseMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        def _parse_locker_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        locker_id = _parse_locker_id(d.pop("locker_id", UNSET))

        def _parse_frm_routing_algorithm(
            data: object,
        ) -> Union[
            "RoutingAlgorithmType0",
            "RoutingAlgorithmType1",
            "RoutingAlgorithmType2",
            "RoutingAlgorithmType3",
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
                componentsschemas_routing_algorithm_type_0 = RoutingAlgorithmType0.from_dict(data)

                return componentsschemas_routing_algorithm_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_1 = RoutingAlgorithmType1.from_dict(data)

                return componentsschemas_routing_algorithm_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_2 = RoutingAlgorithmType2.from_dict(data)

                return componentsschemas_routing_algorithm_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_3 = RoutingAlgorithmType3.from_dict(data)

                return componentsschemas_routing_algorithm_type_3
            except:  # noqa: E722
                pass
            return cast(
                Union[
                    "RoutingAlgorithmType0",
                    "RoutingAlgorithmType1",
                    "RoutingAlgorithmType2",
                    "RoutingAlgorithmType3",
                    None,
                    Unset,
                ],
                data,
            )

        frm_routing_algorithm = _parse_frm_routing_algorithm(d.pop("frm_routing_algorithm", UNSET))

        def _parse_default_profile(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        default_profile = _parse_default_profile(d.pop("default_profile", UNSET))

        def _parse_pm_collect_link_config(data: object) -> Union["BusinessCollectLinkConfig", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                pm_collect_link_config_type_1 = BusinessCollectLinkConfig.from_dict(data)

                return pm_collect_link_config_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BusinessCollectLinkConfig", None, Unset], data)

        pm_collect_link_config = _parse_pm_collect_link_config(d.pop("pm_collect_link_config", UNSET))

        def _parse_product_type(data: object) -> Union[MerchantProductType, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                product_type_type_1 = MerchantProductType(data)

                return product_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[MerchantProductType, None, Unset], data)

        product_type = _parse_product_type(d.pop("product_type", UNSET))

        merchant_account_response = cls(
            merchant_id=merchant_id,
            enable_payment_response_hash=enable_payment_response_hash,
            redirect_to_merchant_with_http_post=redirect_to_merchant_with_http_post,
            primary_business_details=primary_business_details,
            organization_id=organization_id,
            is_recon_enabled=is_recon_enabled,
            recon_status=recon_status,
            merchant_name=merchant_name,
            return_url=return_url,
            payment_response_hash_key=payment_response_hash_key,
            merchant_details=merchant_details,
            webhook_details=webhook_details,
            payout_routing_algorithm=payout_routing_algorithm,
            sub_merchants_enabled=sub_merchants_enabled,
            parent_merchant_id=parent_merchant_id,
            publishable_key=publishable_key,
            metadata=metadata,
            locker_id=locker_id,
            frm_routing_algorithm=frm_routing_algorithm,
            default_profile=default_profile,
            pm_collect_link_config=pm_collect_link_config,
            product_type=product_type,
        )

        merchant_account_response.additional_properties = d
        return merchant_account_response

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
