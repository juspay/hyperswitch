from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.authentication_type import AuthenticationType
from ..models.capture_method import CaptureMethod
from ..models.connector import Connector
from ..models.country_alpha_2 import CountryAlpha2
from ..models.currency import Currency
from ..models.future_usage import FutureUsage
from ..models.payment_experience import PaymentExperience
from ..models.payment_method import PaymentMethod
from ..models.payment_method_type import PaymentMethodType
from ..models.payment_type import PaymentType
from ..models.sca_exemption_type import ScaExemptionType
from ..models.three_ds_completion_indicator import ThreeDsCompletionIndicator
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.address import Address
    from ..models.browser_information import BrowserInformation
    from ..models.connector_metadata import ConnectorMetadata
    from ..models.ctp_service_details import CtpServiceDetails
    from ..models.customer_acceptance import CustomerAcceptance
    from ..models.customer_details import CustomerDetails
    from ..models.mandate_data import MandateData
    from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap
    from ..models.order_details_with_amount import OrderDetailsWithAmount
    from ..models.payment_create_payment_link_config import PaymentCreatePaymentLinkConfig
    from ..models.payment_method_data_request import PaymentMethodDataRequest
    from ..models.payments_create_request_frm_metadata_type_0 import PaymentsCreateRequestFrmMetadataType0
    from ..models.payments_create_request_metadata_type_0 import PaymentsCreateRequestMetadataType0
    from ..models.priority import Priority
    from ..models.recurring_details_type_0 import RecurringDetailsType0
    from ..models.recurring_details_type_1 import RecurringDetailsType1
    from ..models.recurring_details_type_2 import RecurringDetailsType2
    from ..models.recurring_details_type_3 import RecurringDetailsType3
    from ..models.request_surcharge_details import RequestSurchargeDetails
    from ..models.single import Single
    from ..models.split_payments_request_type_0 import SplitPaymentsRequestType0
    from ..models.split_payments_request_type_1 import SplitPaymentsRequestType1
    from ..models.split_payments_request_type_2 import SplitPaymentsRequestType2
    from ..models.volume_split import VolumeSplit


T = TypeVar("T", bound="PaymentsCreateRequest")


@_attrs_define
class PaymentsCreateRequest:
    """
    Attributes:
        amount (int): The payment amount. Amount for the payment in the lowest denomination of the currency, (i.e) in
            cents for USD denomination, in yen for JPY denomination etc. E.g., Pass 100 to charge $1.00 and 1 for 1¥ since ¥
            is a zero-decimal currency. Read more about [the Decimal and Non-Decimal
            Currencies](https://github.com/juspay/hyperswitch/wiki/Decimal-and-Non%E2%80%90Decimal-Currencies)
        currency (Currency): The three letter ISO currency code in uppercase. Eg: 'USD' for the United States Dollar.
        order_tax_amount (Union[None, Unset, int]): Total tax amount applicable to the order Example: 6540.
        amount_to_capture (Union[None, Unset, int]): The Amount to be captured / debited from the users payment method.
            It shall be in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR
            denomination etc., If not provided, the default amount_to_capture will be the payment amount. Also, it must be
            less than or equal to the original payment account. Example: 6540.
        shipping_cost (Union[None, Unset, int]): The shipping cost for the payment. This is required for tax calculation
            in some regions. Example: 6540.
        payment_id (Union[None, Unset, str]): Unique identifier for the payment. This ensures idempotency for multiple
            payments
            that have been done by a single merchant. The value for this field can be specified in the request, it will be
            auto generated otherwise and returned in the API response. Example: pay_mbabizu24mvu3mela5njyhpit4.
        routing (Union['Priority', 'Single', 'VolumeSplit', None, Unset]):
        connector (Union[None, Unset, list[Connector]]): This allows to manually select a connector with which the
            payment can go through. Example: ['stripe', 'adyen'].
        capture_method (Union[CaptureMethod, None, Unset]):
        authentication_type (Union[AuthenticationType, None, Unset]):  Default: AuthenticationType.THREE_DS.
        billing (Union['Address', None, Unset]):
        confirm (Union[None, Unset, bool]): Whether to confirm the payment (if applicable). It can be used to completely
            process a payment by attaching a payment method, setting `confirm=true` and `capture_method = automatic` in the
            *Payments/Create API* request itself. Default: False. Example: True.
        customer (Union['CustomerDetails', None, Unset]):
        customer_id (Union[None, Unset, str]): The identifier for the customer Example: cus_y3oqhf46pyzuxjbcn2giaqnb44.
        off_session (Union[None, Unset, bool]): Set to true to indicate that the customer is not in your checkout flow
            during this payment, and therefore is unable to authenticate. This parameter is intended for scenarios where you
            collect card details and charge them later. When making a recurring payment by passing a mandate_id, this
            parameter is mandatory Example: True.
        description (Union[None, Unset, str]): A description for the payment Example: It's my first payment request.
        return_url (Union[None, Unset, str]): The URL to which you want the user to be redirected after the completion
            of the payment operation Example: https://hyperswitch.io.
        setup_future_usage (Union[FutureUsage, None, Unset]):
        payment_method_data (Union['PaymentMethodDataRequest', None, Unset]):
        payment_method (Union[None, PaymentMethod, Unset]):
        payment_token (Union[None, Unset, str]): As Hyperswitch tokenises the sensitive details about the payments
            method, it provides the payment_token as a reference to a stored payment method, ensuring that the sensitive
            details are not exposed in any manner. Example: 187282ab-40ef-47a9-9206-5099ba31e432.
        shipping (Union['Address', None, Unset]):
        statement_descriptor_name (Union[None, Unset, str]): For non-card charges, you can use this value as the
            complete description that appears on your customers’ statements. Must contain at least one letter, maximum 22
            characters. Example: Hyperswitch Router.
        statement_descriptor_suffix (Union[None, Unset, str]): Provides information about a card payment that customers
            see on their statements. Concatenated with the prefix (shortened descriptor) or statement descriptor that’s set
            on the account to form the complete statement descriptor. Maximum 22 characters for the concatenated descriptor.
            Example: Payment for shoes purchase.
        order_details (Union[None, Unset, list['OrderDetailsWithAmount']]): Use this object to capture the details about
            the different products for which the payment is being made. The sum of amount across different products here
            should be equal to the overall payment amount Example: [{
                    "product_name": "Apple iPhone 16",
                    "quantity": 1,
                    "amount" : 69000
                    "product_img_link" : "https://dummy-img-link.com"
                }].
        mandate_data (Union['MandateData', None, Unset]):
        customer_acceptance (Union['CustomerAcceptance', None, Unset]):
        mandate_id (Union[None, Unset, str]): A unique identifier to link the payment to a mandate. To do Recurring
            payments after a mandate has been created, pass the mandate_id instead of payment_method_data Example:
            mandate_iwer89rnjef349dni3.
        browser_info (Union['BrowserInformation', None, Unset]):
        payment_experience (Union[None, PaymentExperience, Unset]):
        payment_method_type (Union[None, PaymentMethodType, Unset]):
        business_country (Union[CountryAlpha2, None, Unset]):
        business_label (Union[None, Unset, str]): Business label of the merchant for this payment.
            To be deprecated soon. Pass the profile_id instead Example: food.
        merchant_connector_details (Union['MerchantConnectorDetailsWrap', None, Unset]):
        allowed_payment_method_types (Union[None, Unset, list[PaymentMethodType]]): Use this parameter to restrict the
            Payment Method Types to show for a given PaymentIntent
        metadata (Union['PaymentsCreateRequestMetadataType0', None, Unset]): You can specify up to 50 keys, with key
            names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional,
            structured information on an object.
        connector_metadata (Union['ConnectorMetadata', None, Unset]):
        payment_link (Union[None, Unset, bool]): Whether to generate the payment link for this payment or not (if
            applicable) Default: False. Example: True.
        payment_link_config (Union['PaymentCreatePaymentLinkConfig', None, Unset]):
        payment_link_config_id (Union[None, Unset, str]): Custom payment link config id set at business profile, send
            only if business_specific_configs is configured
        profile_id (Union[None, Unset, str]): The business profile to be used for this payment, if not passed the
            default business profile associated with the merchant account will be used. It is mandatory in case multiple
            business profiles have been set up.
        surcharge_details (Union['RequestSurchargeDetails', None, Unset]):
        payment_type (Union[None, PaymentType, Unset]):
        request_incremental_authorization (Union[None, Unset, bool]): Request an incremental authorization, i.e.,
            increase the authorized amount on a confirmed payment before you capture it.
        session_expiry (Union[None, Unset, int]): Will be used to expire client secret after certain amount of time to
            be supplied in seconds
            (900) for 15 mins Example: 900.
        frm_metadata (Union['PaymentsCreateRequestFrmMetadataType0', None, Unset]): Additional data related to some
            frm(Fraud Risk Management) connectors
        request_external_three_ds_authentication (Union[None, Unset, bool]): Whether to perform external authentication
            (if applicable) Example: True.
        recurring_details (Union['RecurringDetailsType0', 'RecurringDetailsType1', 'RecurringDetailsType2',
            'RecurringDetailsType3', None, Unset]):
        split_payments (Union['SplitPaymentsRequestType0', 'SplitPaymentsRequestType1', 'SplitPaymentsRequestType2',
            None, Unset]):
        request_extended_authorization (Union[None, Unset, bool]): Optional boolean value to extent authorization period
            of this payment

            capture method must be manual or manual_multiple Default: False.
        merchant_order_reference_id (Union[None, Unset, str]): Merchant's identifier for the payment/invoice. This will
            be sent to the connector
            if the connector provides support to accept multiple reference ids.
            In case the connector supports only one reference id, Hyperswitch's Payment ID will be sent as reference.
            Example: Custom_Order_id_123.
        skip_external_tax_calculation (Union[None, Unset, bool]): Whether to calculate tax for this payment intent
        psd2_sca_exemption_type (Union[None, ScaExemptionType, Unset]):
        ctp_service_details (Union['CtpServiceDetails', None, Unset]):
        force_3ds_challenge (Union[None, Unset, bool]): Indicates if 3ds challenge is forced
        threeds_method_comp_ind (Union[None, ThreeDsCompletionIndicator, Unset]):
    """

    amount: int
    currency: Currency
    order_tax_amount: Union[None, Unset, int] = UNSET
    amount_to_capture: Union[None, Unset, int] = UNSET
    shipping_cost: Union[None, Unset, int] = UNSET
    payment_id: Union[None, Unset, str] = UNSET
    routing: Union["Priority", "Single", "VolumeSplit", None, Unset] = UNSET
    connector: Union[None, Unset, list[Connector]] = UNSET
    capture_method: Union[CaptureMethod, None, Unset] = UNSET
    authentication_type: Union[AuthenticationType, None, Unset] = AuthenticationType.THREE_DS
    billing: Union["Address", None, Unset] = UNSET
    confirm: Union[None, Unset, bool] = False
    customer: Union["CustomerDetails", None, Unset] = UNSET
    customer_id: Union[None, Unset, str] = UNSET
    off_session: Union[None, Unset, bool] = UNSET
    description: Union[None, Unset, str] = UNSET
    return_url: Union[None, Unset, str] = UNSET
    setup_future_usage: Union[FutureUsage, None, Unset] = UNSET
    payment_method_data: Union["PaymentMethodDataRequest", None, Unset] = UNSET
    payment_method: Union[None, PaymentMethod, Unset] = UNSET
    payment_token: Union[None, Unset, str] = UNSET
    shipping: Union["Address", None, Unset] = UNSET
    statement_descriptor_name: Union[None, Unset, str] = UNSET
    statement_descriptor_suffix: Union[None, Unset, str] = UNSET
    order_details: Union[None, Unset, list["OrderDetailsWithAmount"]] = UNSET
    mandate_data: Union["MandateData", None, Unset] = UNSET
    customer_acceptance: Union["CustomerAcceptance", None, Unset] = UNSET
    mandate_id: Union[None, Unset, str] = UNSET
    browser_info: Union["BrowserInformation", None, Unset] = UNSET
    payment_experience: Union[None, PaymentExperience, Unset] = UNSET
    payment_method_type: Union[None, PaymentMethodType, Unset] = UNSET
    business_country: Union[CountryAlpha2, None, Unset] = UNSET
    business_label: Union[None, Unset, str] = UNSET
    merchant_connector_details: Union["MerchantConnectorDetailsWrap", None, Unset] = UNSET
    allowed_payment_method_types: Union[None, Unset, list[PaymentMethodType]] = UNSET
    metadata: Union["PaymentsCreateRequestMetadataType0", None, Unset] = UNSET
    connector_metadata: Union["ConnectorMetadata", None, Unset] = UNSET
    payment_link: Union[None, Unset, bool] = False
    payment_link_config: Union["PaymentCreatePaymentLinkConfig", None, Unset] = UNSET
    payment_link_config_id: Union[None, Unset, str] = UNSET
    profile_id: Union[None, Unset, str] = UNSET
    surcharge_details: Union["RequestSurchargeDetails", None, Unset] = UNSET
    payment_type: Union[None, PaymentType, Unset] = UNSET
    request_incremental_authorization: Union[None, Unset, bool] = UNSET
    session_expiry: Union[None, Unset, int] = UNSET
    frm_metadata: Union["PaymentsCreateRequestFrmMetadataType0", None, Unset] = UNSET
    request_external_three_ds_authentication: Union[None, Unset, bool] = UNSET
    recurring_details: Union[
        "RecurringDetailsType0", "RecurringDetailsType1", "RecurringDetailsType2", "RecurringDetailsType3", None, Unset
    ] = UNSET
    split_payments: Union[
        "SplitPaymentsRequestType0", "SplitPaymentsRequestType1", "SplitPaymentsRequestType2", None, Unset
    ] = UNSET
    request_extended_authorization: Union[None, Unset, bool] = False
    merchant_order_reference_id: Union[None, Unset, str] = UNSET
    skip_external_tax_calculation: Union[None, Unset, bool] = UNSET
    psd2_sca_exemption_type: Union[None, ScaExemptionType, Unset] = UNSET
    ctp_service_details: Union["CtpServiceDetails", None, Unset] = UNSET
    force_3ds_challenge: Union[None, Unset, bool] = UNSET
    threeds_method_comp_ind: Union[None, ThreeDsCompletionIndicator, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.address import Address
        from ..models.browser_information import BrowserInformation
        from ..models.connector_metadata import ConnectorMetadata
        from ..models.ctp_service_details import CtpServiceDetails
        from ..models.customer_acceptance import CustomerAcceptance
        from ..models.customer_details import CustomerDetails
        from ..models.mandate_data import MandateData
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap
        from ..models.payment_create_payment_link_config import PaymentCreatePaymentLinkConfig
        from ..models.payment_method_data_request import PaymentMethodDataRequest
        from ..models.payments_create_request_frm_metadata_type_0 import PaymentsCreateRequestFrmMetadataType0
        from ..models.payments_create_request_metadata_type_0 import PaymentsCreateRequestMetadataType0
        from ..models.priority import Priority
        from ..models.recurring_details_type_0 import RecurringDetailsType0
        from ..models.recurring_details_type_1 import RecurringDetailsType1
        from ..models.recurring_details_type_2 import RecurringDetailsType2
        from ..models.recurring_details_type_3 import RecurringDetailsType3
        from ..models.request_surcharge_details import RequestSurchargeDetails
        from ..models.single import Single
        from ..models.split_payments_request_type_0 import SplitPaymentsRequestType0
        from ..models.split_payments_request_type_1 import SplitPaymentsRequestType1
        from ..models.split_payments_request_type_2 import SplitPaymentsRequestType2
        from ..models.volume_split import VolumeSplit

        amount = self.amount

        currency = self.currency.value

        order_tax_amount: Union[None, Unset, int]
        if isinstance(self.order_tax_amount, Unset):
            order_tax_amount = UNSET
        else:
            order_tax_amount = self.order_tax_amount

        amount_to_capture: Union[None, Unset, int]
        if isinstance(self.amount_to_capture, Unset):
            amount_to_capture = UNSET
        else:
            amount_to_capture = self.amount_to_capture

        shipping_cost: Union[None, Unset, int]
        if isinstance(self.shipping_cost, Unset):
            shipping_cost = UNSET
        else:
            shipping_cost = self.shipping_cost

        payment_id: Union[None, Unset, str]
        if isinstance(self.payment_id, Unset):
            payment_id = UNSET
        else:
            payment_id = self.payment_id

        routing: Union[None, Unset, dict[str, Any]]
        if isinstance(self.routing, Unset):
            routing = UNSET
        elif isinstance(self.routing, Single):
            routing = self.routing.to_dict()
        elif isinstance(self.routing, Priority):
            routing = self.routing.to_dict()
        elif isinstance(self.routing, VolumeSplit):
            routing = self.routing.to_dict()
        else:
            routing = self.routing

        connector: Union[None, Unset, list[str]]
        if isinstance(self.connector, Unset):
            connector = UNSET
        elif isinstance(self.connector, list):
            connector = []
            for connector_type_0_item_data in self.connector:
                connector_type_0_item = connector_type_0_item_data.value
                connector.append(connector_type_0_item)

        else:
            connector = self.connector

        capture_method: Union[None, Unset, str]
        if isinstance(self.capture_method, Unset):
            capture_method = UNSET
        elif isinstance(self.capture_method, CaptureMethod):
            capture_method = self.capture_method.value
        else:
            capture_method = self.capture_method

        authentication_type: Union[None, Unset, str]
        if isinstance(self.authentication_type, Unset):
            authentication_type = UNSET
        elif isinstance(self.authentication_type, AuthenticationType):
            authentication_type = self.authentication_type.value
        else:
            authentication_type = self.authentication_type

        billing: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing, Unset):
            billing = UNSET
        elif isinstance(self.billing, Address):
            billing = self.billing.to_dict()
        else:
            billing = self.billing

        confirm: Union[None, Unset, bool]
        if isinstance(self.confirm, Unset):
            confirm = UNSET
        else:
            confirm = self.confirm

        customer: Union[None, Unset, dict[str, Any]]
        if isinstance(self.customer, Unset):
            customer = UNSET
        elif isinstance(self.customer, CustomerDetails):
            customer = self.customer.to_dict()
        else:
            customer = self.customer

        customer_id: Union[None, Unset, str]
        if isinstance(self.customer_id, Unset):
            customer_id = UNSET
        else:
            customer_id = self.customer_id

        off_session: Union[None, Unset, bool]
        if isinstance(self.off_session, Unset):
            off_session = UNSET
        else:
            off_session = self.off_session

        description: Union[None, Unset, str]
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        return_url: Union[None, Unset, str]
        if isinstance(self.return_url, Unset):
            return_url = UNSET
        else:
            return_url = self.return_url

        setup_future_usage: Union[None, Unset, str]
        if isinstance(self.setup_future_usage, Unset):
            setup_future_usage = UNSET
        elif isinstance(self.setup_future_usage, FutureUsage):
            setup_future_usage = self.setup_future_usage.value
        else:
            setup_future_usage = self.setup_future_usage

        payment_method_data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payment_method_data, Unset):
            payment_method_data = UNSET
        elif isinstance(self.payment_method_data, PaymentMethodDataRequest):
            payment_method_data = self.payment_method_data.to_dict()
        else:
            payment_method_data = self.payment_method_data

        payment_method: Union[None, Unset, str]
        if isinstance(self.payment_method, Unset):
            payment_method = UNSET
        elif isinstance(self.payment_method, PaymentMethod):
            payment_method = self.payment_method.value
        else:
            payment_method = self.payment_method

        payment_token: Union[None, Unset, str]
        if isinstance(self.payment_token, Unset):
            payment_token = UNSET
        else:
            payment_token = self.payment_token

        shipping: Union[None, Unset, dict[str, Any]]
        if isinstance(self.shipping, Unset):
            shipping = UNSET
        elif isinstance(self.shipping, Address):
            shipping = self.shipping.to_dict()
        else:
            shipping = self.shipping

        statement_descriptor_name: Union[None, Unset, str]
        if isinstance(self.statement_descriptor_name, Unset):
            statement_descriptor_name = UNSET
        else:
            statement_descriptor_name = self.statement_descriptor_name

        statement_descriptor_suffix: Union[None, Unset, str]
        if isinstance(self.statement_descriptor_suffix, Unset):
            statement_descriptor_suffix = UNSET
        else:
            statement_descriptor_suffix = self.statement_descriptor_suffix

        order_details: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.order_details, Unset):
            order_details = UNSET
        elif isinstance(self.order_details, list):
            order_details = []
            for order_details_type_0_item_data in self.order_details:
                order_details_type_0_item = order_details_type_0_item_data.to_dict()
                order_details.append(order_details_type_0_item)

        else:
            order_details = self.order_details

        mandate_data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.mandate_data, Unset):
            mandate_data = UNSET
        elif isinstance(self.mandate_data, MandateData):
            mandate_data = self.mandate_data.to_dict()
        else:
            mandate_data = self.mandate_data

        customer_acceptance: Union[None, Unset, dict[str, Any]]
        if isinstance(self.customer_acceptance, Unset):
            customer_acceptance = UNSET
        elif isinstance(self.customer_acceptance, CustomerAcceptance):
            customer_acceptance = self.customer_acceptance.to_dict()
        else:
            customer_acceptance = self.customer_acceptance

        mandate_id: Union[None, Unset, str]
        if isinstance(self.mandate_id, Unset):
            mandate_id = UNSET
        else:
            mandate_id = self.mandate_id

        browser_info: Union[None, Unset, dict[str, Any]]
        if isinstance(self.browser_info, Unset):
            browser_info = UNSET
        elif isinstance(self.browser_info, BrowserInformation):
            browser_info = self.browser_info.to_dict()
        else:
            browser_info = self.browser_info

        payment_experience: Union[None, Unset, str]
        if isinstance(self.payment_experience, Unset):
            payment_experience = UNSET
        elif isinstance(self.payment_experience, PaymentExperience):
            payment_experience = self.payment_experience.value
        else:
            payment_experience = self.payment_experience

        payment_method_type: Union[None, Unset, str]
        if isinstance(self.payment_method_type, Unset):
            payment_method_type = UNSET
        elif isinstance(self.payment_method_type, PaymentMethodType):
            payment_method_type = self.payment_method_type.value
        else:
            payment_method_type = self.payment_method_type

        business_country: Union[None, Unset, str]
        if isinstance(self.business_country, Unset):
            business_country = UNSET
        elif isinstance(self.business_country, CountryAlpha2):
            business_country = self.business_country.value
        else:
            business_country = self.business_country

        business_label: Union[None, Unset, str]
        if isinstance(self.business_label, Unset):
            business_label = UNSET
        else:
            business_label = self.business_label

        merchant_connector_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.merchant_connector_details, Unset):
            merchant_connector_details = UNSET
        elif isinstance(self.merchant_connector_details, MerchantConnectorDetailsWrap):
            merchant_connector_details = self.merchant_connector_details.to_dict()
        else:
            merchant_connector_details = self.merchant_connector_details

        allowed_payment_method_types: Union[None, Unset, list[str]]
        if isinstance(self.allowed_payment_method_types, Unset):
            allowed_payment_method_types = UNSET
        elif isinstance(self.allowed_payment_method_types, list):
            allowed_payment_method_types = []
            for allowed_payment_method_types_type_0_item_data in self.allowed_payment_method_types:
                allowed_payment_method_types_type_0_item = allowed_payment_method_types_type_0_item_data.value
                allowed_payment_method_types.append(allowed_payment_method_types_type_0_item)

        else:
            allowed_payment_method_types = self.allowed_payment_method_types

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, PaymentsCreateRequestMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        connector_metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.connector_metadata, Unset):
            connector_metadata = UNSET
        elif isinstance(self.connector_metadata, ConnectorMetadata):
            connector_metadata = self.connector_metadata.to_dict()
        else:
            connector_metadata = self.connector_metadata

        payment_link: Union[None, Unset, bool]
        if isinstance(self.payment_link, Unset):
            payment_link = UNSET
        else:
            payment_link = self.payment_link

        payment_link_config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payment_link_config, Unset):
            payment_link_config = UNSET
        elif isinstance(self.payment_link_config, PaymentCreatePaymentLinkConfig):
            payment_link_config = self.payment_link_config.to_dict()
        else:
            payment_link_config = self.payment_link_config

        payment_link_config_id: Union[None, Unset, str]
        if isinstance(self.payment_link_config_id, Unset):
            payment_link_config_id = UNSET
        else:
            payment_link_config_id = self.payment_link_config_id

        profile_id: Union[None, Unset, str]
        if isinstance(self.profile_id, Unset):
            profile_id = UNSET
        else:
            profile_id = self.profile_id

        surcharge_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.surcharge_details, Unset):
            surcharge_details = UNSET
        elif isinstance(self.surcharge_details, RequestSurchargeDetails):
            surcharge_details = self.surcharge_details.to_dict()
        else:
            surcharge_details = self.surcharge_details

        payment_type: Union[None, Unset, str]
        if isinstance(self.payment_type, Unset):
            payment_type = UNSET
        elif isinstance(self.payment_type, PaymentType):
            payment_type = self.payment_type.value
        else:
            payment_type = self.payment_type

        request_incremental_authorization: Union[None, Unset, bool]
        if isinstance(self.request_incremental_authorization, Unset):
            request_incremental_authorization = UNSET
        else:
            request_incremental_authorization = self.request_incremental_authorization

        session_expiry: Union[None, Unset, int]
        if isinstance(self.session_expiry, Unset):
            session_expiry = UNSET
        else:
            session_expiry = self.session_expiry

        frm_metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.frm_metadata, Unset):
            frm_metadata = UNSET
        elif isinstance(self.frm_metadata, PaymentsCreateRequestFrmMetadataType0):
            frm_metadata = self.frm_metadata.to_dict()
        else:
            frm_metadata = self.frm_metadata

        request_external_three_ds_authentication: Union[None, Unset, bool]
        if isinstance(self.request_external_three_ds_authentication, Unset):
            request_external_three_ds_authentication = UNSET
        else:
            request_external_three_ds_authentication = self.request_external_three_ds_authentication

        recurring_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.recurring_details, Unset):
            recurring_details = UNSET
        elif isinstance(self.recurring_details, RecurringDetailsType0):
            recurring_details = self.recurring_details.to_dict()
        elif isinstance(self.recurring_details, RecurringDetailsType1):
            recurring_details = self.recurring_details.to_dict()
        elif isinstance(self.recurring_details, RecurringDetailsType2):
            recurring_details = self.recurring_details.to_dict()
        elif isinstance(self.recurring_details, RecurringDetailsType3):
            recurring_details = self.recurring_details.to_dict()
        else:
            recurring_details = self.recurring_details

        split_payments: Union[None, Unset, dict[str, Any]]
        if isinstance(self.split_payments, Unset):
            split_payments = UNSET
        elif isinstance(self.split_payments, SplitPaymentsRequestType0):
            split_payments = self.split_payments.to_dict()
        elif isinstance(self.split_payments, SplitPaymentsRequestType1):
            split_payments = self.split_payments.to_dict()
        elif isinstance(self.split_payments, SplitPaymentsRequestType2):
            split_payments = self.split_payments.to_dict()
        else:
            split_payments = self.split_payments

        request_extended_authorization: Union[None, Unset, bool]
        if isinstance(self.request_extended_authorization, Unset):
            request_extended_authorization = UNSET
        else:
            request_extended_authorization = self.request_extended_authorization

        merchant_order_reference_id: Union[None, Unset, str]
        if isinstance(self.merchant_order_reference_id, Unset):
            merchant_order_reference_id = UNSET
        else:
            merchant_order_reference_id = self.merchant_order_reference_id

        skip_external_tax_calculation: Union[None, Unset, bool]
        if isinstance(self.skip_external_tax_calculation, Unset):
            skip_external_tax_calculation = UNSET
        else:
            skip_external_tax_calculation = self.skip_external_tax_calculation

        psd2_sca_exemption_type: Union[None, Unset, str]
        if isinstance(self.psd2_sca_exemption_type, Unset):
            psd2_sca_exemption_type = UNSET
        elif isinstance(self.psd2_sca_exemption_type, ScaExemptionType):
            psd2_sca_exemption_type = self.psd2_sca_exemption_type.value
        else:
            psd2_sca_exemption_type = self.psd2_sca_exemption_type

        ctp_service_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.ctp_service_details, Unset):
            ctp_service_details = UNSET
        elif isinstance(self.ctp_service_details, CtpServiceDetails):
            ctp_service_details = self.ctp_service_details.to_dict()
        else:
            ctp_service_details = self.ctp_service_details

        force_3ds_challenge: Union[None, Unset, bool]
        if isinstance(self.force_3ds_challenge, Unset):
            force_3ds_challenge = UNSET
        else:
            force_3ds_challenge = self.force_3ds_challenge

        threeds_method_comp_ind: Union[None, Unset, str]
        if isinstance(self.threeds_method_comp_ind, Unset):
            threeds_method_comp_ind = UNSET
        elif isinstance(self.threeds_method_comp_ind, ThreeDsCompletionIndicator):
            threeds_method_comp_ind = self.threeds_method_comp_ind.value
        else:
            threeds_method_comp_ind = self.threeds_method_comp_ind

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "amount": amount,
                "currency": currency,
            }
        )
        if order_tax_amount is not UNSET:
            field_dict["order_tax_amount"] = order_tax_amount
        if amount_to_capture is not UNSET:
            field_dict["amount_to_capture"] = amount_to_capture
        if shipping_cost is not UNSET:
            field_dict["shipping_cost"] = shipping_cost
        if payment_id is not UNSET:
            field_dict["payment_id"] = payment_id
        if routing is not UNSET:
            field_dict["routing"] = routing
        if connector is not UNSET:
            field_dict["connector"] = connector
        if capture_method is not UNSET:
            field_dict["capture_method"] = capture_method
        if authentication_type is not UNSET:
            field_dict["authentication_type"] = authentication_type
        if billing is not UNSET:
            field_dict["billing"] = billing
        if confirm is not UNSET:
            field_dict["confirm"] = confirm
        if customer is not UNSET:
            field_dict["customer"] = customer
        if customer_id is not UNSET:
            field_dict["customer_id"] = customer_id
        if off_session is not UNSET:
            field_dict["off_session"] = off_session
        if description is not UNSET:
            field_dict["description"] = description
        if return_url is not UNSET:
            field_dict["return_url"] = return_url
        if setup_future_usage is not UNSET:
            field_dict["setup_future_usage"] = setup_future_usage
        if payment_method_data is not UNSET:
            field_dict["payment_method_data"] = payment_method_data
        if payment_method is not UNSET:
            field_dict["payment_method"] = payment_method
        if payment_token is not UNSET:
            field_dict["payment_token"] = payment_token
        if shipping is not UNSET:
            field_dict["shipping"] = shipping
        if statement_descriptor_name is not UNSET:
            field_dict["statement_descriptor_name"] = statement_descriptor_name
        if statement_descriptor_suffix is not UNSET:
            field_dict["statement_descriptor_suffix"] = statement_descriptor_suffix
        if order_details is not UNSET:
            field_dict["order_details"] = order_details
        if mandate_data is not UNSET:
            field_dict["mandate_data"] = mandate_data
        if customer_acceptance is not UNSET:
            field_dict["customer_acceptance"] = customer_acceptance
        if mandate_id is not UNSET:
            field_dict["mandate_id"] = mandate_id
        if browser_info is not UNSET:
            field_dict["browser_info"] = browser_info
        if payment_experience is not UNSET:
            field_dict["payment_experience"] = payment_experience
        if payment_method_type is not UNSET:
            field_dict["payment_method_type"] = payment_method_type
        if business_country is not UNSET:
            field_dict["business_country"] = business_country
        if business_label is not UNSET:
            field_dict["business_label"] = business_label
        if merchant_connector_details is not UNSET:
            field_dict["merchant_connector_details"] = merchant_connector_details
        if allowed_payment_method_types is not UNSET:
            field_dict["allowed_payment_method_types"] = allowed_payment_method_types
        if metadata is not UNSET:
            field_dict["metadata"] = metadata
        if connector_metadata is not UNSET:
            field_dict["connector_metadata"] = connector_metadata
        if payment_link is not UNSET:
            field_dict["payment_link"] = payment_link
        if payment_link_config is not UNSET:
            field_dict["payment_link_config"] = payment_link_config
        if payment_link_config_id is not UNSET:
            field_dict["payment_link_config_id"] = payment_link_config_id
        if profile_id is not UNSET:
            field_dict["profile_id"] = profile_id
        if surcharge_details is not UNSET:
            field_dict["surcharge_details"] = surcharge_details
        if payment_type is not UNSET:
            field_dict["payment_type"] = payment_type
        if request_incremental_authorization is not UNSET:
            field_dict["request_incremental_authorization"] = request_incremental_authorization
        if session_expiry is not UNSET:
            field_dict["session_expiry"] = session_expiry
        if frm_metadata is not UNSET:
            field_dict["frm_metadata"] = frm_metadata
        if request_external_three_ds_authentication is not UNSET:
            field_dict["request_external_three_ds_authentication"] = request_external_three_ds_authentication
        if recurring_details is not UNSET:
            field_dict["recurring_details"] = recurring_details
        if split_payments is not UNSET:
            field_dict["split_payments"] = split_payments
        if request_extended_authorization is not UNSET:
            field_dict["request_extended_authorization"] = request_extended_authorization
        if merchant_order_reference_id is not UNSET:
            field_dict["merchant_order_reference_id"] = merchant_order_reference_id
        if skip_external_tax_calculation is not UNSET:
            field_dict["skip_external_tax_calculation"] = skip_external_tax_calculation
        if psd2_sca_exemption_type is not UNSET:
            field_dict["psd2_sca_exemption_type"] = psd2_sca_exemption_type
        if ctp_service_details is not UNSET:
            field_dict["ctp_service_details"] = ctp_service_details
        if force_3ds_challenge is not UNSET:
            field_dict["force_3ds_challenge"] = force_3ds_challenge
        if threeds_method_comp_ind is not UNSET:
            field_dict["threeds_method_comp_ind"] = threeds_method_comp_ind

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.address import Address
        from ..models.browser_information import BrowserInformation
        from ..models.connector_metadata import ConnectorMetadata
        from ..models.ctp_service_details import CtpServiceDetails
        from ..models.customer_acceptance import CustomerAcceptance
        from ..models.customer_details import CustomerDetails
        from ..models.mandate_data import MandateData
        from ..models.merchant_connector_details_wrap import MerchantConnectorDetailsWrap
        from ..models.order_details_with_amount import OrderDetailsWithAmount
        from ..models.payment_create_payment_link_config import PaymentCreatePaymentLinkConfig
        from ..models.payment_method_data_request import PaymentMethodDataRequest
        from ..models.payments_create_request_frm_metadata_type_0 import PaymentsCreateRequestFrmMetadataType0
        from ..models.payments_create_request_metadata_type_0 import PaymentsCreateRequestMetadataType0
        from ..models.priority import Priority
        from ..models.recurring_details_type_0 import RecurringDetailsType0
        from ..models.recurring_details_type_1 import RecurringDetailsType1
        from ..models.recurring_details_type_2 import RecurringDetailsType2
        from ..models.recurring_details_type_3 import RecurringDetailsType3
        from ..models.request_surcharge_details import RequestSurchargeDetails
        from ..models.single import Single
        from ..models.split_payments_request_type_0 import SplitPaymentsRequestType0
        from ..models.split_payments_request_type_1 import SplitPaymentsRequestType1
        from ..models.split_payments_request_type_2 import SplitPaymentsRequestType2
        from ..models.volume_split import VolumeSplit

        d = dict(src_dict)
        amount = d.pop("amount")

        currency = Currency(d.pop("currency"))

        def _parse_order_tax_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        order_tax_amount = _parse_order_tax_amount(d.pop("order_tax_amount", UNSET))

        def _parse_amount_to_capture(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        amount_to_capture = _parse_amount_to_capture(d.pop("amount_to_capture", UNSET))

        def _parse_shipping_cost(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        shipping_cost = _parse_shipping_cost(d.pop("shipping_cost", UNSET))

        def _parse_payment_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_id = _parse_payment_id(d.pop("payment_id", UNSET))

        def _parse_routing(data: object) -> Union["Priority", "Single", "VolumeSplit", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_straight_through_algorithm_type_0 = Single.from_dict(data)

                return componentsschemas_straight_through_algorithm_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_straight_through_algorithm_type_1 = Priority.from_dict(data)

                return componentsschemas_straight_through_algorithm_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_straight_through_algorithm_type_2 = VolumeSplit.from_dict(data)

                return componentsschemas_straight_through_algorithm_type_2
            except:  # noqa: E722
                pass
            return cast(Union["Priority", "Single", "VolumeSplit", None, Unset], data)

        routing = _parse_routing(d.pop("routing", UNSET))

        def _parse_connector(data: object) -> Union[None, Unset, list[Connector]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                connector_type_0 = []
                _connector_type_0 = data
                for connector_type_0_item_data in _connector_type_0:
                    connector_type_0_item = Connector(connector_type_0_item_data)

                    connector_type_0.append(connector_type_0_item)

                return connector_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[Connector]], data)

        connector = _parse_connector(d.pop("connector", UNSET))

        def _parse_capture_method(data: object) -> Union[CaptureMethod, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                capture_method_type_1 = CaptureMethod(data)

                return capture_method_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CaptureMethod, None, Unset], data)

        capture_method = _parse_capture_method(d.pop("capture_method", UNSET))

        def _parse_authentication_type(data: object) -> Union[AuthenticationType, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                authentication_type_type_1 = AuthenticationType(data)

                return authentication_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[AuthenticationType, None, Unset], data)

        authentication_type = _parse_authentication_type(d.pop("authentication_type", UNSET))

        def _parse_billing(data: object) -> Union["Address", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                billing_type_1 = Address.from_dict(data)

                return billing_type_1
            except:  # noqa: E722
                pass
            return cast(Union["Address", None, Unset], data)

        billing = _parse_billing(d.pop("billing", UNSET))

        def _parse_confirm(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        confirm = _parse_confirm(d.pop("confirm", UNSET))

        def _parse_customer(data: object) -> Union["CustomerDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                customer_type_1 = CustomerDetails.from_dict(data)

                return customer_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CustomerDetails", None, Unset], data)

        customer = _parse_customer(d.pop("customer", UNSET))

        def _parse_customer_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        customer_id = _parse_customer_id(d.pop("customer_id", UNSET))

        def _parse_off_session(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        off_session = _parse_off_session(d.pop("off_session", UNSET))

        def _parse_description(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        description = _parse_description(d.pop("description", UNSET))

        def _parse_return_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        return_url = _parse_return_url(d.pop("return_url", UNSET))

        def _parse_setup_future_usage(data: object) -> Union[FutureUsage, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                setup_future_usage_type_1 = FutureUsage(data)

                return setup_future_usage_type_1
            except:  # noqa: E722
                pass
            return cast(Union[FutureUsage, None, Unset], data)

        setup_future_usage = _parse_setup_future_usage(d.pop("setup_future_usage", UNSET))

        def _parse_payment_method_data(data: object) -> Union["PaymentMethodDataRequest", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                payment_method_data_type_1 = PaymentMethodDataRequest.from_dict(data)

                return payment_method_data_type_1
            except:  # noqa: E722
                pass
            return cast(Union["PaymentMethodDataRequest", None, Unset], data)

        payment_method_data = _parse_payment_method_data(d.pop("payment_method_data", UNSET))

        def _parse_payment_method(data: object) -> Union[None, PaymentMethod, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_method_type_1 = PaymentMethod(data)

                return payment_method_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentMethod, Unset], data)

        payment_method = _parse_payment_method(d.pop("payment_method", UNSET))

        def _parse_payment_token(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_token = _parse_payment_token(d.pop("payment_token", UNSET))

        def _parse_shipping(data: object) -> Union["Address", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                shipping_type_1 = Address.from_dict(data)

                return shipping_type_1
            except:  # noqa: E722
                pass
            return cast(Union["Address", None, Unset], data)

        shipping = _parse_shipping(d.pop("shipping", UNSET))

        def _parse_statement_descriptor_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        statement_descriptor_name = _parse_statement_descriptor_name(d.pop("statement_descriptor_name", UNSET))

        def _parse_statement_descriptor_suffix(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        statement_descriptor_suffix = _parse_statement_descriptor_suffix(d.pop("statement_descriptor_suffix", UNSET))

        def _parse_order_details(data: object) -> Union[None, Unset, list["OrderDetailsWithAmount"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                order_details_type_0 = []
                _order_details_type_0 = data
                for order_details_type_0_item_data in _order_details_type_0:
                    order_details_type_0_item = OrderDetailsWithAmount.from_dict(order_details_type_0_item_data)

                    order_details_type_0.append(order_details_type_0_item)

                return order_details_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["OrderDetailsWithAmount"]], data)

        order_details = _parse_order_details(d.pop("order_details", UNSET))

        def _parse_mandate_data(data: object) -> Union["MandateData", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                mandate_data_type_1 = MandateData.from_dict(data)

                return mandate_data_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MandateData", None, Unset], data)

        mandate_data = _parse_mandate_data(d.pop("mandate_data", UNSET))

        def _parse_customer_acceptance(data: object) -> Union["CustomerAcceptance", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                customer_acceptance_type_1 = CustomerAcceptance.from_dict(data)

                return customer_acceptance_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CustomerAcceptance", None, Unset], data)

        customer_acceptance = _parse_customer_acceptance(d.pop("customer_acceptance", UNSET))

        def _parse_mandate_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        mandate_id = _parse_mandate_id(d.pop("mandate_id", UNSET))

        def _parse_browser_info(data: object) -> Union["BrowserInformation", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                browser_info_type_1 = BrowserInformation.from_dict(data)

                return browser_info_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BrowserInformation", None, Unset], data)

        browser_info = _parse_browser_info(d.pop("browser_info", UNSET))

        def _parse_payment_experience(data: object) -> Union[None, PaymentExperience, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_experience_type_1 = PaymentExperience(data)

                return payment_experience_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentExperience, Unset], data)

        payment_experience = _parse_payment_experience(d.pop("payment_experience", UNSET))

        def _parse_payment_method_type(data: object) -> Union[None, PaymentMethodType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_method_type_type_1 = PaymentMethodType(data)

                return payment_method_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentMethodType, Unset], data)

        payment_method_type = _parse_payment_method_type(d.pop("payment_method_type", UNSET))

        def _parse_business_country(data: object) -> Union[CountryAlpha2, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                business_country_type_1 = CountryAlpha2(data)

                return business_country_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CountryAlpha2, None, Unset], data)

        business_country = _parse_business_country(d.pop("business_country", UNSET))

        def _parse_business_label(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        business_label = _parse_business_label(d.pop("business_label", UNSET))

        def _parse_merchant_connector_details(data: object) -> Union["MerchantConnectorDetailsWrap", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                merchant_connector_details_type_1 = MerchantConnectorDetailsWrap.from_dict(data)

                return merchant_connector_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorDetailsWrap", None, Unset], data)

        merchant_connector_details = _parse_merchant_connector_details(d.pop("merchant_connector_details", UNSET))

        def _parse_allowed_payment_method_types(data: object) -> Union[None, Unset, list[PaymentMethodType]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                allowed_payment_method_types_type_0 = []
                _allowed_payment_method_types_type_0 = data
                for allowed_payment_method_types_type_0_item_data in _allowed_payment_method_types_type_0:
                    allowed_payment_method_types_type_0_item = PaymentMethodType(
                        allowed_payment_method_types_type_0_item_data
                    )

                    allowed_payment_method_types_type_0.append(allowed_payment_method_types_type_0_item)

                return allowed_payment_method_types_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[PaymentMethodType]], data)

        allowed_payment_method_types = _parse_allowed_payment_method_types(d.pop("allowed_payment_method_types", UNSET))

        def _parse_metadata(data: object) -> Union["PaymentsCreateRequestMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = PaymentsCreateRequestMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["PaymentsCreateRequestMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        def _parse_connector_metadata(data: object) -> Union["ConnectorMetadata", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                connector_metadata_type_1 = ConnectorMetadata.from_dict(data)

                return connector_metadata_type_1
            except:  # noqa: E722
                pass
            return cast(Union["ConnectorMetadata", None, Unset], data)

        connector_metadata = _parse_connector_metadata(d.pop("connector_metadata", UNSET))

        def _parse_payment_link(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        payment_link = _parse_payment_link(d.pop("payment_link", UNSET))

        def _parse_payment_link_config(data: object) -> Union["PaymentCreatePaymentLinkConfig", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                payment_link_config_type_1 = PaymentCreatePaymentLinkConfig.from_dict(data)

                return payment_link_config_type_1
            except:  # noqa: E722
                pass
            return cast(Union["PaymentCreatePaymentLinkConfig", None, Unset], data)

        payment_link_config = _parse_payment_link_config(d.pop("payment_link_config", UNSET))

        def _parse_payment_link_config_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_link_config_id = _parse_payment_link_config_id(d.pop("payment_link_config_id", UNSET))

        def _parse_profile_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        profile_id = _parse_profile_id(d.pop("profile_id", UNSET))

        def _parse_surcharge_details(data: object) -> Union["RequestSurchargeDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                surcharge_details_type_1 = RequestSurchargeDetails.from_dict(data)

                return surcharge_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["RequestSurchargeDetails", None, Unset], data)

        surcharge_details = _parse_surcharge_details(d.pop("surcharge_details", UNSET))

        def _parse_payment_type(data: object) -> Union[None, PaymentType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_type_type_1 = PaymentType(data)

                return payment_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentType, Unset], data)

        payment_type = _parse_payment_type(d.pop("payment_type", UNSET))

        def _parse_request_incremental_authorization(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        request_incremental_authorization = _parse_request_incremental_authorization(
            d.pop("request_incremental_authorization", UNSET)
        )

        def _parse_session_expiry(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        session_expiry = _parse_session_expiry(d.pop("session_expiry", UNSET))

        def _parse_frm_metadata(data: object) -> Union["PaymentsCreateRequestFrmMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                frm_metadata_type_0 = PaymentsCreateRequestFrmMetadataType0.from_dict(data)

                return frm_metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["PaymentsCreateRequestFrmMetadataType0", None, Unset], data)

        frm_metadata = _parse_frm_metadata(d.pop("frm_metadata", UNSET))

        def _parse_request_external_three_ds_authentication(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        request_external_three_ds_authentication = _parse_request_external_three_ds_authentication(
            d.pop("request_external_three_ds_authentication", UNSET)
        )

        def _parse_recurring_details(
            data: object,
        ) -> Union[
            "RecurringDetailsType0",
            "RecurringDetailsType1",
            "RecurringDetailsType2",
            "RecurringDetailsType3",
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
                componentsschemas_recurring_details_type_0 = RecurringDetailsType0.from_dict(data)

                return componentsschemas_recurring_details_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_recurring_details_type_1 = RecurringDetailsType1.from_dict(data)

                return componentsschemas_recurring_details_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_recurring_details_type_2 = RecurringDetailsType2.from_dict(data)

                return componentsschemas_recurring_details_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_recurring_details_type_3 = RecurringDetailsType3.from_dict(data)

                return componentsschemas_recurring_details_type_3
            except:  # noqa: E722
                pass
            return cast(
                Union[
                    "RecurringDetailsType0",
                    "RecurringDetailsType1",
                    "RecurringDetailsType2",
                    "RecurringDetailsType3",
                    None,
                    Unset,
                ],
                data,
            )

        recurring_details = _parse_recurring_details(d.pop("recurring_details", UNSET))

        def _parse_split_payments(
            data: object,
        ) -> Union["SplitPaymentsRequestType0", "SplitPaymentsRequestType1", "SplitPaymentsRequestType2", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_split_payments_request_type_0 = SplitPaymentsRequestType0.from_dict(data)

                return componentsschemas_split_payments_request_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_split_payments_request_type_1 = SplitPaymentsRequestType1.from_dict(data)

                return componentsschemas_split_payments_request_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_split_payments_request_type_2 = SplitPaymentsRequestType2.from_dict(data)

                return componentsschemas_split_payments_request_type_2
            except:  # noqa: E722
                pass
            return cast(
                Union[
                    "SplitPaymentsRequestType0", "SplitPaymentsRequestType1", "SplitPaymentsRequestType2", None, Unset
                ],
                data,
            )

        split_payments = _parse_split_payments(d.pop("split_payments", UNSET))

        def _parse_request_extended_authorization(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        request_extended_authorization = _parse_request_extended_authorization(
            d.pop("request_extended_authorization", UNSET)
        )

        def _parse_merchant_order_reference_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_order_reference_id = _parse_merchant_order_reference_id(d.pop("merchant_order_reference_id", UNSET))

        def _parse_skip_external_tax_calculation(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        skip_external_tax_calculation = _parse_skip_external_tax_calculation(
            d.pop("skip_external_tax_calculation", UNSET)
        )

        def _parse_psd2_sca_exemption_type(data: object) -> Union[None, ScaExemptionType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                psd2_sca_exemption_type_type_1 = ScaExemptionType(data)

                return psd2_sca_exemption_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, ScaExemptionType, Unset], data)

        psd2_sca_exemption_type = _parse_psd2_sca_exemption_type(d.pop("psd2_sca_exemption_type", UNSET))

        def _parse_ctp_service_details(data: object) -> Union["CtpServiceDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                ctp_service_details_type_1 = CtpServiceDetails.from_dict(data)

                return ctp_service_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CtpServiceDetails", None, Unset], data)

        ctp_service_details = _parse_ctp_service_details(d.pop("ctp_service_details", UNSET))

        def _parse_force_3ds_challenge(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        force_3ds_challenge = _parse_force_3ds_challenge(d.pop("force_3ds_challenge", UNSET))

        def _parse_threeds_method_comp_ind(data: object) -> Union[None, ThreeDsCompletionIndicator, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                threeds_method_comp_ind_type_1 = ThreeDsCompletionIndicator(data)

                return threeds_method_comp_ind_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, ThreeDsCompletionIndicator, Unset], data)

        threeds_method_comp_ind = _parse_threeds_method_comp_ind(d.pop("threeds_method_comp_ind", UNSET))

        payments_create_request = cls(
            amount=amount,
            currency=currency,
            order_tax_amount=order_tax_amount,
            amount_to_capture=amount_to_capture,
            shipping_cost=shipping_cost,
            payment_id=payment_id,
            routing=routing,
            connector=connector,
            capture_method=capture_method,
            authentication_type=authentication_type,
            billing=billing,
            confirm=confirm,
            customer=customer,
            customer_id=customer_id,
            off_session=off_session,
            description=description,
            return_url=return_url,
            setup_future_usage=setup_future_usage,
            payment_method_data=payment_method_data,
            payment_method=payment_method,
            payment_token=payment_token,
            shipping=shipping,
            statement_descriptor_name=statement_descriptor_name,
            statement_descriptor_suffix=statement_descriptor_suffix,
            order_details=order_details,
            mandate_data=mandate_data,
            customer_acceptance=customer_acceptance,
            mandate_id=mandate_id,
            browser_info=browser_info,
            payment_experience=payment_experience,
            payment_method_type=payment_method_type,
            business_country=business_country,
            business_label=business_label,
            merchant_connector_details=merchant_connector_details,
            allowed_payment_method_types=allowed_payment_method_types,
            metadata=metadata,
            connector_metadata=connector_metadata,
            payment_link=payment_link,
            payment_link_config=payment_link_config,
            payment_link_config_id=payment_link_config_id,
            profile_id=profile_id,
            surcharge_details=surcharge_details,
            payment_type=payment_type,
            request_incremental_authorization=request_incremental_authorization,
            session_expiry=session_expiry,
            frm_metadata=frm_metadata,
            request_external_three_ds_authentication=request_external_three_ds_authentication,
            recurring_details=recurring_details,
            split_payments=split_payments,
            request_extended_authorization=request_extended_authorization,
            merchant_order_reference_id=merchant_order_reference_id,
            skip_external_tax_calculation=skip_external_tax_calculation,
            psd2_sca_exemption_type=psd2_sca_exemption_type,
            ctp_service_details=ctp_service_details,
            force_3ds_challenge=force_3ds_challenge,
            threeds_method_comp_ind=threeds_method_comp_ind,
        )

        payments_create_request.additional_properties = d
        return payments_create_request

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
