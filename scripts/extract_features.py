#!/usr/bin/env python3
"""
Hyperswitch Feature Extraction Script

Scans the codebase to generate 3 CSV files:
  - bucket_1_connector_features.csv  (connector + flow features)
  - bucket_2_connector_pm_features.csv (connector + PM + PMT features)
  - bucket_3_core_features.csv (core features)

Detection methods:
  1. default_implementations.rs: connectors NOT in default macro = real impl
  2. ConnectorSpecifications trait overrides in connector code
  3. connector_enums.rs: is_*/should_* methods on Connector enum
  4. Transformer field usage in connector transformers
  5. SupportedPaymentMethods static for Bucket 2
  6. Cypress test config files for coverage mapping

Usage:
  python3 scripts/extract_features.py
"""

import re
import os
import glob
import sys
import sqlite3
from collections import defaultdict

# ---- Configuration ----
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CONN_DIR = os.path.join(REPO_ROOT, "crates/hyperswitch_connectors/src/connectors")
DEFAULT_IMPL_FILE = os.path.join(REPO_ROOT, "crates/hyperswitch_connectors/src/default_implementations.rs")
CONNECTOR_ENUMS_FILE = os.path.join(REPO_ROOT, "crates/common_enums/src/connector_enums.rs")
CYPRESS_DIR = os.path.join(REPO_ROOT, "cypress-tests/cypress/e2e/configs/Payment")
CYPRESS_UTILS = os.path.join(CYPRESS_DIR, "Utils.js")
CYPRESS_SPEC_ROOT = os.path.join(REPO_ROOT, "cypress-tests/cypress/e2e/spec")
API_RS = os.path.join(REPO_ROOT, "crates/hyperswitch_interfaces/src/api.rs")
DB_PATH = os.path.join(REPO_ROOT, "features.db")

# ---- Exclusions ----
# Connectors to completely exclude from feature extraction
EXCLUDED_CONNECTORS = {
    # Can't test in hosted environment
    "absa_sanlam",
    # Standard exclusions
    "blackhawknetwork", "boku", "breadpay", "celero", "chargebee", "digitalvirgo",
    "flexiti", "getnet", "gpayments", "hyperwallet", "imerchantsolutions",
    "juspaythreedsserver", "katapult", "mpgs", "payeezy", "paytm", "phonepe",
    "powertranz", "prophetpay", "santander", "sift", "silverflow", "square",
    "hyperpg", "tokenex", "trustpayments", "zen"
}

# Payment method types to exclude from Bucket 2
EXCLUDED_PM_TYPES_BUCKET2 = {"GooglePay", "ApplePay"}

# Specific connector + flow combinations to exclude
EXCLUDED_FLOW_COMBINATIONS = {
    ("airwallex", "Order Create Flow"),   # Internal flow
    ("nordea",    "Order Create Flow"),   # Internal flow
    ("payme",     "Order Create Flow"),   # Internal flow
    ("razorpay",  "Order Create Flow"),   # Internal flow
    ("trustpay",  "Order Create Flow"),   # Internal flow
    ("amazonpay", "Refund"),              # Not possible to verify e2e cases
    ("bitpay",    "Refund"),              # Not possible to verify e2e cases
    ("coingate",  "Refund"),              # Not possible to verify e2e cases
    ("gigadat",   "Refund"),              # Not possible to verify e2e cases
    ("itaubank",  "Refund"),              # Not possible to verify e2e cases
    ("klarna",    "Refund"),              # Not possible to verify e2e cases
    ("loonio",    "Refund"),              # Not possible to verify e2e cases
    ("razorpay",  "Refund"),              # Not possible to verify e2e cases
    ("santander", "Refund"),              # Not possible to verify e2e cases
    ("stripe",    "Overcapture"),         # Creds not available
    ("revolv3",   "Refund"),              # UCS only connector
    ("truelayer", "Refund"),              # UCS only connector
    ("trustly",   "Refund"),              # UCS only connector
    ("adyen",     "Split Refunds"),       # Creds not available
    ("adyen",          "Dispute Accept"), # No connector config data
    ("adyen",          "Dispute Defend"), # No connector config data
    ("checkout",       "Dispute Accept"), # No connector config data
    ("checkout",       "Dispute Defend"), # No connector config data
    ("worldpayvantiv", "Dispute Accept"), # No connector config data
    ("adyen",          "Split Payments"), # Creds not available
    ("xendit",         "Split Payments"), # Creds not available
}

# Features to exclude from Bucket 3
EXCLUDED_FEATURES_BUCKET3 = {
    "Split Transactions Enabled",    # v2 feature
    "Process Tracker Mapping",       # Not testable via Cypress
    "Payout Tracker Mapping",        # Not testable via Cypress
    "CVV Collection During Payment", # v2 feature
    "Dispute Polling Interval",      # Not possible in Cypress
    "FRM Routing Algorithm",         # Internal flow
    "Sub-Merchants",                 # Deprecated feature
}


def get_all_connectors():
    """Get all connector module names from the connectors directory."""
    connectors = set()
    for entry in os.listdir(CONN_DIR):
        path = os.path.join(CONN_DIR, entry)
        if os.path.isdir(path) and entry not in ("__pycache__",):
            connectors.add(entry)
        elif entry.endswith(".rs") and entry not in ("mod.rs",):
            connectors.add(entry.replace(".rs", ""))
    # Exclude non-connector modules
    connectors.discard("mod")
    connectors.discard("utils")
    return sorted(connectors)


def parse_default_implementations():
    """
    Parse default_implementations.rs to find which connectors get no-op impls per flow.
    Returns: dict[macro_name] -> set of connector names (lowercase)
    """
    with open(DEFAULT_IMPL_FILE) as f:
        content = f.read()

    result = {}
    # Find each macro invocation: default_imp_for_X!(\n connectors::A,\n connectors::B,\n ...);
    pattern = r'(default_imp_for_\w+)!\(\s*((?:.*?\n)*?)\s*\);'
    for match in re.finditer(pattern, content):
        macro_name = match.group(1)
        body = match.group(2)
        connectors = set()
        for c in re.findall(r'connectors::(\w+)', body):
            connectors.add(c.lower())
        result[macro_name] = connectors

    return result


# Mapping from default macro name -> feature name and endpoint
FLOW_MACRO_MAP = {
    "default_imp_for_incremental_authorization": {
        "feature": "Incremental Authorization",
        "description": "Increase the authorized amount after initial authorization without re-authenticating the cardholder",
        "endpoint": "POST /payments/{id}/incremental_authorization",
    },
    "default_imp_for_extend_authorization": {
        "feature": "Extended Authorization",
        "description": "Extend the authorization hold window beyond the connector's default expiry period",
        "endpoint": "POST /payments/{id}/extend_authorization",
    },
    "default_imp_for_pre_authenticate_steps": {
        "feature": "Pre-Authentication Flow",
        "description": "3DS enrollment check — queries whether the card is enrolled in 3DS before initiating a challenge",
        "endpoint": "POST /payments (internal 3DS enrollment check)",
    },
    "default_imp_for_authenticate_steps": {
        "feature": "Authentication Flow",
        "description": "3DS challenge step — presents OTP or biometric prompt to cardholder and processes the authentication result",
        "endpoint": "POST /payments (internal 3DS challenge)",
    },
    "default_imp_for_post_authenticate_steps": {
        "feature": "Post-Authentication Flow",
        "description": "Complete authorization after 3DS challenge result is received and verified",
        "endpoint": "POST /payments/{id}/complete_authorize",
    },
    "default_imp_for_create_order": {
        "feature": "Order Create Flow",
        "description": "Connector-specific order creation step that must occur before the authorization call (e.g. Klarna/BNPL order setup)",
        "endpoint": "POST /payments (internal order creation before auth)",
    },
    "default_imp_for_payment_settlement_split_create": {
        "feature": "Settlement Split Call",
        "description": "Split settlement amounts across sub-merchants or accounts after authorization",
        "endpoint": "POST /payments (internal post-auth settlement split)",
    },
    "default_imp_for_generate_qr_flow": {
        "feature": "QR Code Generation Flow",
        "description": "Generate a scannable QR code for the payment (e.g. PIX, WeChat Pay, GrabPay)",
        "endpoint": "POST /payments (internal QR code generation)",
    },
    "default_imp_for_push_notification_flow": {
        "feature": "Push Notification Flow",
        "description": "Handle bank-initiated push notification to trigger a new mandate charge cycle (e.g. PIX Automático)",
        "endpoint": "POST /payments (internal mandate push notification)",
    },
    "default_imp_for_gift_card_balance_check": {
        "feature": "Balance Check Flow",
        "description": "Check remaining balance on a gift card before authorizing the payment",
        "endpoint": "POST /payments (internal gift card balance check)",
    },
    "default_imp_for_accept_dispute": {
        "feature": "Dispute Accept",
        "description": "Accept a chargeback dispute without contesting — connector-specific API call to the dispute service",
        "endpoint": "POST /disputes/{id}/accept",
    },
    "default_imp_for_defend_dispute": {
        "feature": "Dispute Defend",
        "description": "Submit evidence documents to defend against a chargeback dispute at the connector",
        "endpoint": "POST /disputes/{id}/evidence",
    },
    "default_imp_for_revenue_recovery": {
        "feature": "Revenue Recovery",
        "description": "Automatic retry of failed subscription payments via a billing connector (Chargebee, Recurly, StripeBilling)",
        "endpoint": "POST /payments (internal subscription payment retry)",
    },
}


def detect_features_via_default_impls(all_connectors, default_impls):
    """
    For each flow macro, find connectors that are NOT in the default list
    = they have real implementations.
    """
    rows = []
    for macro_name, info in sorted(FLOW_MACRO_MAP.items()):
        if macro_name not in default_impls:
            continue
        default_connectors = default_impls[macro_name]
        real_connectors = sorted(
            c for c in all_connectors if c.lower() not in default_connectors
        )
        # Filter out non-payment connectors
        skip = {"dummyconnector", "ctp_mastercard", "opayo", "recurly",
                "chargebee", "stripebilling", "custombilling", "taxjar",
                "riskified", "sift", "signifyd", "gpayments", "netcetera",
                "threedsecureio", "juspaythreedsserver",
                "unified_authentication_service", "cybersourcedecisionmanager",
                "vgs", "hyperswitch_vault"}
        # For some features, billing/FRM connectors ARE relevant
        billing_features = {"Revenue Recovery"}
        if info["feature"] not in billing_features:
            real_connectors = [c for c in real_connectors if c not in skip]
        else:
            # For revenue recovery, only include billing connectors
            billing = {"chargebee", "stripebilling", "recurly"}
            real_connectors = [c for c in real_connectors if c in billing]

        for c in real_connectors:
            rows.append((c, info["feature"], info["description"], info["endpoint"]))
    return rows


def detect_connector_enum_features():
    """Parse connector_enums.rs for is_*/should_* methods on Connector enum."""
    with open(CONNECTOR_ENUMS_FILE) as f:
        content = f.read()

    rows = []

    # Overcapture
    m = re.search(r'is_overcapture_supported_by_connector.*?matches!\(self,\s*(.*?)\)', content, re.DOTALL)
    if m:
        connectors = re.findall(r'Self::(\w+)', m.group(1))
        for c in connectors:
            rows.append((c.lower(), "Overcapture",
                         "Capture more than the originally authorized amount (overcapture / over-capture)",
                         "POST /payments/{id}/capture (amount > authorized)"))

    return rows


def detect_connector_spec_overrides():
    """
    Search connector code for ConnectorSpecifications method overrides
    that return non-default values.
    """
    rows = []
    methods = {
        "should_call_connector_customer": {
            "feature": "Connector Customer Creation",
            "endpoint": "POST /payments (internal connector customer create)",
            "default": "false",
        },
        "should_call_tokenization_before_setup_mandate": {
            "feature": "Skip Tokenization Before Mandate",
            "endpoint": "POST /payments (internal skip tokenization on mandate setup)",
            "default": "true",
        },
        "should_trigger_handle_response_without_body": {
            "feature": "Handle Response Without Body",
            "endpoint": "POST /payments (internal handle response without connector call)",
            "default": "false",
        },
        "authentication_token_for_token_creation": {
            "feature": "Auth Token For Token Creation",
            "endpoint": "POST /payments (internal auth token before access token)",
            "default": "false",
        },
        "is_authorize_session_token_call_required": {
            "feature": "Authorize Session Token",
            "endpoint": "POST /payments/session_tokens (pre-auth session token)",
            "default": "false",
        },
        "generate_connector_customer_id": {
            "feature": "Connector Customer ID Generation",
            "endpoint": "POST /payments (internal custom customer ID format)",
            "default": "None",
        },
        "generate_connector_request_reference_id": {
            "feature": "Connector Request Reference ID",
            "endpoint": "POST /payments (internal custom reference ID format)",
            "default": None,  # complex default, just detect override
        },
        "is_payment_recurrence_operation_needed": {
            "feature": "Payment Recurrence Operation",
            "endpoint": "POST /payments (internal recurring operation check)",
            "default": "Some(false)",
        },
        "get_api_webhook_config": {
            "feature": "API Webhook Config",
            "endpoint": "POST /account/{account_id}/connectors (webhook setup capabilities)",
            "default": None,
        },
    }

    for f in sorted(glob.glob(f"{CONN_DIR}/**/*.rs", recursive=True) + glob.glob(f"{CONN_DIR}/*.rs")):
        with open(f) as fh:
            content = fh.read()
        rel = f.replace(CONN_DIR + "/", "")
        connector = rel.split("/")[0].replace(".rs", "")
        if connector in ("mod", "utils"):
            continue

        for method_name, info in methods.items():
            if method_name in content and f"fn {method_name}" in content:
                # Verify it's an actual override (in ConnectorSpecifications impl)
                if re.search(rf'fn\s+{method_name}\s*\(', content):
                    rows.append((connector, info["feature"], info["endpoint"]))

    return rows


def detect_transformer_features():
    """
    Search connector transformers for specific field usage indicating
    feature support.
    """
    features = {
        "partial_auth": {
            "patterns": [r'partial_auth.*true|PartialAuth\w+\s*\{|allow_partial|PartialApprovalFlag|is_partial_approval'],
            "exclude_patterns": [r'partial_auth.*None|enable_partial_auth.*None'],
            "feature": "Partial Authorization",
            "description": "Allow connector to partially approve a payment when the full amount is unavailable (partial approval)",
            "endpoint": "POST /payments (enable_partial_authorization=true)",
        },
        "split_payments": {
            "patterns": [r'SplitPayment|StripeSplit|AdyenSplit|XenditSplit|split_payments'],
            "feature": "Split Payments",
            "description": "Split a single payment across multiple accounts or sub-merchant recipients",
            "endpoint": "POST /payments (split_payments in body)",
        },
        "split_refunds": {
            "patterns": [r'split_refund|SplitRefund'],
            "feature": "Split Refunds",
            "description": "Refund a split payment proportionally across the original split recipients",
            "endpoint": "POST /refunds (split_refunds in body)",
        },
        "billing_descriptor": {
            "patterns": [r'billing_descriptor'],
            "feature": "Billing Descriptor",
            "description": "Custom text shown on the cardholder's bank statement (soft descriptor / statement descriptor)",
            "endpoint": "POST /payments (billing_descriptor in body)",
            "transformer_only": True,
        },
        "l2_l3": {
            "patterns": [r'l2_l3|L2L3|level_two|level_three|LevelTwo|LevelThree'],
            "feature": "L2/L3 Data Processing",
            "description": "Send Level 2/3 line-item data (tax, PO number, product codes) for commercial card interchange optimization",
            "endpoint": "POST /payments (order_details with L2/L3 fields)",
            "transformer_only": True,
        },
        "partner_merchant": {
            "patterns": [r'partner_merchant'],
            "feature": "Partner Merchant Identifier",
            "description": "Identify the platform/marketplace and sub-merchant in the connector request (e.g. Adyen ApplicationInfo)",
            "endpoint": "POST /payments (partner_merchant_identifier_details in body)",
            "transformer_only": True,
        },
        "surcharge": {
            "patterns": [r'surcharge'],
            "feature": "Surcharge",
            "description": "Add a connector-forwarded surcharge fee on top of the payment amount",
            "endpoint": "POST /payments (surcharge_details in body)",
            "transformer_only": True,
        },
        "installments": {
            "patterns": [r'installment'],
            "feature": "Installments",
            "description": "Split the payment into fixed monthly installments at the connector level",
            "endpoint": "POST /payments (installment_data in body)",
            "transformer_only": True,
        },
        "network_transaction_id": {
            "patterns": [r'network_transaction_id'],
            "feature": "Network Transaction ID",
            "description": "Return the network-assigned transaction ID in the response for use in subsequent MIT/recurring payments",
            "endpoint": "POST /payments (network_transaction_id in response)",
            "transformer_only": True,
        },
        "connector_testing_data": {
            "patterns": [r'connector_testing_data'],
            "feature": "Connector Testing Data",
            "description": "Pass connector-specific testing flags/data in the request for sandbox/test mode behavior",
            "endpoint": "POST /payments (connector_testing_data in router_data)",
            "transformer_only": True,
        },
        "connector_intent_metadata": {
            "patterns": [r'connector_intent_metadata'],
            "feature": "Connector Intent Metadata",
            "description": "Pass opaque connector-specific intent metadata in the request (e.g. PIX Automático max mandate amount)",
            "endpoint": "POST /payments (connector_intent_metadata in router_data)",
            "transformer_only": True,
        },
        "step_up": {
            "patterns": [r'step_up'],
            "feature": "Step Up Authentication",
            "description": "Trigger a 3DS step-up challenge when a prior frictionless flow is deemed insufficient by the issuer",
            "endpoint": "POST /payments (internal 3DS2 step-up redirect)",
            "transformer_only": True,
        },
    }

    rows = []
    for f in sorted(glob.glob(f"{CONN_DIR}/**/*.rs", recursive=True) + glob.glob(f"{CONN_DIR}/*.rs")):
        with open(f) as fh:
            content = fh.read()
        rel = f.replace(CONN_DIR + "/", "")
        connector = rel.split("/")[0].replace(".rs", "")
        if connector in ("mod", "utils"):
            continue

        is_transformer = "transformer" in f.lower()

        for key, info in features.items():
            # If transformer_only, only check transformer files
            if info.get("transformer_only") and not is_transformer:
                continue

            matched = False
            for pat in info["patterns"]:
                if re.search(pat, content):
                    matched = True
                    break

            if matched and "exclude_patterns" in info:
                for pat in info["exclude_patterns"]:
                    # If ONLY excluded patterns match (stub), skip
                    has_real = False
                    for p in info["patterns"]:
                        matches = re.findall(p, content)
                        if matches:
                            has_real = True
                    if not has_real:
                        matched = False

            if matched:
                rows.append((connector, info["feature"], info["description"], info["endpoint"]))

    return rows


def detect_wallet_decrypt_variants():
    """
    Detect wallet payment method types that have a decrypt flow variant,
    i.e. the connector receives pre-decrypted token data from HS before
    forwarding to the connector API.

    Detection: transformer files that match on PaymentMethodToken::ApplePayDecrypt,
    GooglePayDecrypt, or PazeDecrypt.

    Returns list of (connector, payment_method, payment_method_type, feature, description, endpoint).
    """
    _decrypt_desc = "HS pre-decrypts the wallet token and sends plaintext card data to the connector instead of the encrypted blob"
    variants = {
        "PaymentMethodToken::ApplePayDecrypt": ("Wallet", "ApplePay", "Payment (Decrypt Flow)",
                                                _decrypt_desc,
                                                "POST /payments (ApplePay pre-decrypted token)"),
        "PaymentMethodToken::GooglePayDecrypt": ("Wallet", "GooglePay", "Payment (Decrypt Flow)",
                                                 _decrypt_desc,
                                                 "POST /payments (GooglePay pre-decrypted token)"),
        "PaymentMethodToken::PazeDecrypt": ("Wallet", "Paze", "Payment (Decrypt Flow)",
                                            _decrypt_desc,
                                            "POST /payments (Paze pre-decrypted token)"),
    }

    rows = []
    seen = set()
    for f in sorted(glob.glob(f"{CONN_DIR}/**/*.rs", recursive=True) + glob.glob(f"{CONN_DIR}/*.rs")):
        if "transformer" not in f.lower():
            continue
        with open(f) as fh:
            content = fh.read()
        rel = f.replace(CONN_DIR + "/", "")
        connector = rel.split("/")[0].replace(".rs", "")
        if connector in ("mod", "utils"):
            continue

        for token_variant, (pm, pmt, feature, desc, endpoint) in variants.items():
            if token_variant in content:
                key = (connector, pm, pmt, feature)
                if key not in seen:
                    seen.add(key)
                    rows.append((connector, pm, pmt, feature, desc, endpoint))

    return rows


def detect_refund_support():
    """
    Detect per-connector refund support from SUPPORTED_PAYMENT_METHODS.
    A connector supports refunds if ANY (PM, PMT) entry has refunds: Supported.
    Returns list of (connector, feature, description, endpoint).
    """
    _fs = r'(?:(?:common_enums::)?(?:enums::)?)FeatureStatus::'
    _pm = r'(?:(?:common_enums::)?(?:enums::)?)PaymentMethod::'
    _pmt = r'(?:(?:common_enums::)?(?:enums::)?)PaymentMethodType::'
    pattern = re.compile(
        r'\.add\(\s*' + _pm + r'(\w+)\s*,\s*'
        + _pmt + r'\w+\s*,\s*'
        r'PaymentMethodDetails\s*\{\s*'
        r'mandates:\s*' + _fs + r'\w+\s*,\s*'
        r'refunds:\s*' + _fs + r'(\w+)',
    )

    rows = []
    seen = set()
    for f in sorted(glob.glob(f"{CONN_DIR}/**/*.rs", recursive=True) + glob.glob(f"{CONN_DIR}/*.rs")):
        with open(f) as fh:
            content = fh.read()
        if "SUPPORTED_PAYMENT_METHODS" not in content:
            continue
        rel = f.replace(CONN_DIR + "/", "")
        connector = rel.split("/")[0].replace(".rs", "")
        if connector in ("mod", "utils") or connector in seen:
            continue
        adds = pattern.findall(content)
        if any(refund_status == "Supported" for _, refund_status in adds):
            seen.add(connector)
            rows.append((
                connector,
                "Refund",
                "Connector supports refund of a completed payment (at least one PM/PMT combination)",
                "POST /refunds",
            ))
    return rows


def extract_bucket2():
    """
    Extract Bucket 2 features from SupportedPaymentMethods static in each connector.
    Returns only Supported entries.
    """
    entries = set()
    for f in sorted(glob.glob(f"{CONN_DIR}/**/*.rs", recursive=True) + glob.glob(f"{CONN_DIR}/*.rs")):
        with open(f) as fh:
            content = fh.read()
        if "SUPPORTED_PAYMENT_METHODS" not in content:
            continue
        rel = f.replace(CONN_DIR + "/", "")
        connector = rel.split("/")[0].replace(".rs", "")
        if connector in ("mod", "utils"):
            continue

        # Handle all namespace variants:
        #   FeatureStatus::, enums::FeatureStatus::,
        #   common_enums::FeatureStatus::, common_enums::enums::FeatureStatus::
        _fs = r'(?:(?:common_enums::)?(?:enums::)?)FeatureStatus::'
        _pm = r'(?:(?:common_enums::)?(?:enums::)?)PaymentMethod::'
        _pmt = r'(?:(?:common_enums::)?(?:enums::)?)PaymentMethodType::'
        adds = re.findall(
            r'\.add\(\s*' + _pm + r'(\w+)\s*,\s*'
            + _pmt + r'(\w+)\s*,\s*'
            r'PaymentMethodDetails\s*\{\s*'
            r'mandates:\s*' + _fs + r'(\w+)\s*,\s*'
            r'refunds:\s*' + _fs + r'(\w+)',
            content,
        )
        for pm, pmt, mandates, refunds in adds:
            entries.add((connector, pm, pmt, mandates, refunds))

    return sorted(entries)


def parse_cypress_configs():
    """
    Parse cypress test configs to determine which connectors have test
    coverage for which PM types and features.
    """
    configs = {}
    skip = {"Commons.js", "Modifiers.js", "Utils.js"}

    # Cypress files whose tests actually exercise a different connector module
    # than the filename suggests. e.g. StripeConnect.js exercises the `stripe`
    # connector via the split_payments path — there is no `stripeconnect`
    # module in crates/hyperswitch_connectors/.
    file_alias = {
        "stripeconnect": "stripe",
    }

    if not os.path.exists(CYPRESS_DIR):
        return configs

    # Derive the set of PM category strings from the canonical PM_CATEGORY_MAP so
    # there is a single source of truth. Adding a new PM type to PM_CATEGORY_MAP
    # automatically covers B2 detection — no second list to keep in sync.
    pm_categories = set(PM_CATEGORY_MAP.values())

    for fname in os.listdir(CYPRESS_DIR):
        if not fname.endswith(".js") or fname in skip:
            continue
        raw_connector = fname.replace(".js", "").lower()
        connector = file_alias.get(raw_connector, raw_connector)
        fpath = os.path.join(CYPRESS_DIR, fname)
        with open(fpath) as f:
            content = f.read()

        # Which PM categories does this connector config cover?
        pm_types = {cat for cat in pm_categories if cat in content}

        # Global feature keywords (file-wide, used for B1 checks and backwards compat)
        features = set()
        if "Refund" in content:
            features.add("Refund")
        if "Void" in content:
            features.add("Void")
        if "Mandate" in content:
            features.add("Mandate")
        if "SaveCard" in content:
            features.add("SaveCard")
        if "split_payments" in content or "SplitPayment" in content:
            features.add("Split Payments")
        if "split_refunds" in content or "SplitRefund" in content:
            features.add("Split Refunds")

        # Per-PM-section mandate and refund coverage.
        # We scan a window of text starting at each PM section header so that a
        # Mandate test defined only in card_pm does NOT accidentally mark
        # wallet_pm or bank_debit_pm as having mandate coverage.
        # Window size (20 000 chars) is large enough to cover even the biggest
        # PM blocks in the current configs.
        _WINDOW = 20_000
        pm_mandate: set = set()
        pm_refund: set = set()
        pm_pmts: dict = {}   # pm_cat -> set of lowercase PMT names explicitly tested
        for cat in pm_types:
            idx = content.find(f"{cat}:")
            if idx == -1:
                idx = content.find(f"{cat} :")
            if idx == -1:
                continue
            chunk = content[idx: idx + _WINDOW]
            if re.search(r'Mandate\w*\s*:', chunk):
                pm_mandate.add(cat)
            if re.search(r'Refund\s*:', chunk):
                pm_refund.add(cat)
            # For pay_later_pm only: extract explicit payment_method_type values.
            # PayLater has many PMTs (Klarna, Affirm, AfterpayClearpay, etc.) with
            # completely different flows — a connector only testing Klarna should NOT
            # be marked covered for Affirm. For other PM categories (wallet, card,
            # bank_redirect, etc.) the pm_cat presence is a sufficient proxy.
            if cat == "pay_later_pm":
                pmts_found = set(re.findall(r'payment_method_type\s*:\s*["\'](\w+)["\']', chunk))
                if pmts_found:
                    pm_pmts[cat] = pmts_found

        # Merge into existing config if alias was used (e.g. StripeConnect into stripe)
        existing = configs.get(connector, {
            "pm_types": set(), "features": set(),
            "pm_mandate": set(), "pm_refund": set(),
            "pm_pmts": {},
        })
        existing["pm_types"]   |= pm_types
        existing["features"]   |= features
        existing["pm_mandate"] |= pm_mandate
        existing["pm_refund"]  |= pm_refund
        for cat, pmts in pm_pmts.items():
            existing["pm_pmts"].setdefault(cat, set()).update(pmts)
        configs[connector] = existing

    # Parse INCLUDE and EXCLUDE lists from Utils.js
    include_lists = {}
    exclude_lists = {}
    if os.path.exists(CYPRESS_UTILS):
        with open(CYPRESS_UTILS) as f:
            utils_content = f.read()

        for section_name, target in (("INCLUDE", include_lists), ("EXCLUDE", exclude_lists)):
            sec_match = re.search(rf'{section_name}:\s*\{{(.*?)\}},?\s*\n\s*(?://|\}})', utils_content, re.DOTALL)
            if not sec_match:
                continue
            sec_body = sec_match.group(1)
            for list_match in re.finditer(r'(\w+):\s*\[(.*?)\]', sec_body, re.DOTALL):
                name = list_match.group(1)
                connectors = re.findall(r'"(\w+)"', list_match.group(2))
                target[name] = set(c.lower() for c in connectors)

    # Stash EXCLUDE on include_lists under a sentinel key so callers can use it
    # without changing the function signature for every existing call site.
    include_lists["__EXCLUDE__"] = exclude_lists

    return configs, include_lists


PM_CATEGORY_MAP = {
    "Card": "card_pm",
    "BankTransfer": "bank_transfer_pm",
    "BankRedirect": "bank_redirect_pm",
    "Wallet": "wallet_pm",
    "Upi": "upi_pm",
    "Crypto": "crypto_pm",
    "Reward": "reward_pm",
    "PayLater": "pay_later_pm",
    "BankDebit": "bank_debit_pm",
    "Voucher": "voucher_pm",
    "RealTimePayment": "real_time_payment_pm",
    "GiftCard": "gift_card_pm",
    "OpenBanking": "open_banking_pm",
    "MobilePayment": "mobile_payment_pm",
    "CardRedirect": "card_redirect_pm",
    "NetworkToken": "card_pm",
}

# ---------------------------------------------------------------------------
# Utils.js INCLUDE list → B1 feature name mapping
#
# This is the SINGLE source of truth for "which Utils.js INCLUDE list controls
# Cypress coverage for which B1 feature". When a new Cypress PR adds a new
# INCLUDE list to Utils.js, add ONE entry here — that is the only edit required.
#
# Conventions:
#   - B1 connector-specific feature  → set value to the exact feature name string
#   - Multiple lists for the same feature (e.g. BillingDescriptor variants) → both
#     map to the same feature name; the connector is "covered" if it appears in ANY
#   - B3 / B2 / structural lists → None with an inline comment explaining why
#   - MANDATES_USING_NTID_PROXY    → None; handled by custom logic below
# ---------------------------------------------------------------------------
UTILS_INCLUDE_FEATURE_MAP = {
    # ---- B1: per-connector feature coverage ----
    "INCREMENTAL_AUTH":                  "Incremental Authorization",
    "OVERCAPTURE":                       "Overcapture",
    "CARD_INSTALLMENTS":                 "Installments",
    "BILLING_DESCRIPTOR":                "Billing Descriptor",
    "BILLING_DESCRIPTOR_INVALID_PHONE":  "Billing Descriptor",       # variant — same feature
    "EXTERNAL_THREE_DS":                 "External 3DS Authentication",
    "PARTNER_MERCHANT_IDENTIFIER":       "Partner Merchant Identifier",
    "EXTEND_AUTHORIZATION":              "Extended Authorization",
    "STEP_UP_AUTH":                      "Step Up Authentication",
    "STEP_UP_RETRY":                     "Step Up Retry",           # B1: GSM-driven step-up retry
    "PRE_AUTHENTICATION":                "Pre-Authentication Flow", # B1: pre-auth 3DS flow
    "GIFT_CARD":                         "Balance Check Flow",
    "CONNECTOR_TESTING_DATA":            "Connector Testing Data",
    "PARTIAL_AUTH":                      "Partial Authorization",
    "L2L3DATA":                          "L2/L3 Data Processing",

    # ---- Handled by custom Network Transaction ID logic, not this map ----
    "MANDATES_USING_NTID_PROXY":         None,

    # ---- B3: connector-agnostic tests (these lists control test participation,
    #          not per-connector feature flags — B3 status is set globally) ----
    "MANUAL_RETRY":                      None,   # B3: Manual Retry
    "AUTO_RETRY":                        None,   # B3: Auto Retries
    "PAYMENTS_WEBHOOK":                  None,   # B3: Webhook Details
    "REFUNDS_WEBHOOK":                   None,   # B3: Webhook Details
    "AUTH_SERVICE_ELIGIBILITY":          None,   # B3: Authentication Service Eligibility
    "USE_BILLING_AS_PAYMENT_METHOD_BILLING": None,  # B3: Use Billing As PM Billing
    "MIT_WITH_LIMITED_CARD_DATA":        None,   # B3: MIT With Limited Card Data
    "REFUND_MANUAL_UPDATE":              None,   # B3: Refund Manual Update (detected via FEATURE_SPEC_PATTERNS)
    "FEATURE_METADATA":                  None,   # B3: Feature Metadata
    "ORDER_DETAILS":                     None,   # B3: Order Details (spec exists; B3 status set via FEATURE_SPEC_PATTERNS)
    "RELAY_OPERATIONS":                  None,   # B3: Relay Operations (spec exists; B3 status set via FEATURE_SPEC_PATTERNS)

    # ---- B2: PM-level coverage — not a per-connector B1 flag ----
    "BANK_DEBIT":                        None,   # B2: BankDebit PM coverage
    "PAY_LATER":                         None,   # B2: PayLater PM coverage
    "AFFIRM":                            None,   # B2: PayLater/Affirm sub-test (covered via pay_later_pm)
    "ALIPAY_HK_WALLET":                  None,   # B2: Wallet/AliPayHk (already covered via wallet_pm)
    "BLUECODE_WALLET":                   None,   # B2: Wallet/Bluecode (already covered)
    "MIFINITY_WALLET":                   None,   # B2: Wallet/Mifinity (already covered)
    "PAYPAL_WALLET":                     None,   # B2: Wallet/Paypal (already covered)
    "PAYPAL_MANDATE":                    None,   # B2: Wallet/Paypal/Mandate (covered via pm_mandate)

    # ---- B3: B3 feature, detected via FEATURE_SPEC_PATTERNS ----
    "CARD_TESTING_GUARD":                None,   # B3: Card Testing Guard
    "PAYMENT_LINK_CARD":                 None,   # B3: Payment Link (spec detected via PaymentLink pattern)
    "POLL_CONFIG":                       None,   # B3: Poll Config (spec detected via PollConfig pattern)

    # ---- Structural: test infrastructure, no corresponding feature row ----
    "DDC_RACE_CONDITION":                None,   # worldpay DDC timing test
    "UCS_CONNECTORS":                    None,   # Unified Connector Service participants
}


def get_cypress_status_bucket1(connector, feature, cypress_configs, include_lists):
    """Determine cypress coverage for a Bucket 1 (connector, feature) pair."""
    c = connector.lower()
    if c not in cypress_configs:
        return "no_cypress_config"

    # Build feature → set-of-include-list-names from the module-level map.
    # Multiple INCLUDE lists can map to the same feature (e.g. BILLING_DESCRIPTOR
    # and BILLING_DESCRIPTOR_INVALID_PHONE both cover "Billing Descriptor"); a
    # connector is "covered" if it appears in ANY of them.
    feature_to_lists = defaultdict(set)
    for list_name, feat in UTILS_INCLUDE_FEATURE_MAP.items():
        if feat is not None:
            feature_to_lists[feat].add(list_name)

    if feature in feature_to_lists:
        for list_name in feature_to_lists[feature]:
            if list_name in include_lists and c in include_lists[list_name]:
                return "covered"
        return "not_covered"

    # Refund - check if connector cypress config has Refund tests
    if feature == "Refund":
        cfg = cypress_configs.get(c, {})
        return "covered" if "Refund" in cfg.get("features", set()) else "not_covered"

    # Split Payments / Split Refunds — detected from cypress config keywords
    if feature == "Split Payments":
        cfg = cypress_configs.get(c, {})
        return "covered" if "Split Payments" in cfg.get("features", set()) else "not_covered"
    if feature == "Split Refunds":
        cfg = cypress_configs.get(c, {})
        return "covered" if "Split Refunds" in cfg.get("features", set()) else "not_covered"

    # Network Transaction ID — covered if:
    #   (a) connector uses the NTID proxy MIT test (21-MandatesUsingNTIDProxy.cy.js), OR
    #   (b) connector is NOT in the agnostic-NTID exclude list (25-ConnectorAgnosticNTID.cy.js
    #       runs for every connector except those listed)
    if feature == "Network Transaction ID":
        if "MANDATES_USING_NTID_PROXY" in include_lists and c in include_lists["MANDATES_USING_NTID_PROXY"]:
            return "covered"
        agnostic_excluded = include_lists.get("__EXCLUDE__", {}).get("CONNECTOR_AGNOSTIC_NTID", set())
        if c not in agnostic_excluded:
            return "covered"
        return "not_covered"

    return "not_covered"


def get_cypress_status_bucket2(connector, pm, pmt, feature, cypress_configs):
    """Determine cypress coverage for a Bucket 2 (connector, PM, PMT, feature) pair."""
    c = connector.lower()
    if c not in cypress_configs:
        return "no_cypress_config"

    cfg = cypress_configs[c]
    pm_cat = PM_CATEGORY_MAP.get(pm, f"{pm.lower()}_pm")

    if pm_cat not in cfg["pm_types"]:
        return "not_covered"

    if feature == "Payment":
        # For PM categories where configs specify individual PMTs (e.g. pay_later_pm,
        # real_time_payment_pm), check that this exact PMT is configured — not just
        # the PM category.  For PM categories that don't use payment_method_type
        # (e.g. card_pm, wallet_pm), fall back to PM-category-level detection.
        pm_pmts = cfg.get("pm_pmts", {}).get(pm_cat)
        if pm_pmts:
            return "covered" if pmt.lower() in pm_pmts else "not_covered"
        return "covered"

    # Mandate and Refund are checked per-PM-section (not file-wide) so that a
    # Card mandate test does not falsely mark Wallet or BankDebit as covered.
    if feature == "Mandate":
        return "covered" if pm_cat in cfg.get("pm_mandate", set()) else "not_covered"

    if feature == "Refund":
        return "covered" if pm_cat in cfg.get("pm_refund", set()) else "not_covered"

    return "not_covered"


# ---------------------------------------------------------------------------
# Spec-walk based cypress detection (Option 2)
# ---------------------------------------------------------------------------
# Walk every .cy.js file under cypress-tests/cypress/e2e/spec/ recursively,
# pull spec filenames + describe()/it() titles, and match to feature names
# via FEATURE_SPEC_PATTERNS. This is what makes B3 cypress detection
# *actually dynamic* (vs. the hardcoded `cypress_status` in BUCKET_3_FEATURES)
# and it also catches new test categories (like the Routing/ specs added by
# PR #12033) that live outside the original Payment/ tree the parser used to
# scan.

# Each value is a list of regex patterns. A spec matches a feature if any of
# its patterns appears (case-insensitive) in the spec filename, any
# describe() title, or any it() title in that file. Add a new entry when a
# new test category (or include list) lands.
FEATURE_SPEC_PATTERNS = {
    # ---- Routing-family (B3) ----
    "Routing Algorithm":          [r"PriorityRouting", r"VolumeBasedRouting", r"RuleBasedRouting"],
    "Default Fallback Routing":   [r"DefaultRouting", r"FallbackRouting"],
    "Dynamic Routing":            [r"DynamicRouting"],
    "Conditional Routing DSL":    [r"ConditionalRouting", r"RoutingDSL", r"RuleBasedRouting", r"Rule Based Routing"],
    "FRM Routing Algorithm":      [r"FRMRouting", r"FraudRouting"],
    "Payout Routing Algorithm":   [r"PayoutRouting"],
    "3DS Decision Rule Algorithm": [r"3DSDecisionRule", r"ThreeDSDecisionRule"],
    "3DS Routing Region UAS":     [r"ThreeDSRoutingRegion"],
    "Authentication Flow":        [r"ThreeDSAutoCapture", r"ThreeDSManualCapture", r"05-ThreeDS", r"16-ThreeDS", r"ThreeDS"],
    "Routing Result Source":      [r"RoutingResultSource"],
    "Routing Evaluate":           [r"RoutingEvaluate"],

    # ---- Retry-family (B3) ----
    "Auto Retries":               [r"AutoRetries", r"AutoRetry"],
    "Gateway Status Map (GSM)":   [r"AutoRetries", r"AutoRetry"],   # GSM is read by every auto-retry flow
    "Manual Retry":                [r"ManualRetry"],
    "Clear PAN Retries":          [r"ClearPan", r"ClearPanRetry"],

    # ---- Webhook (B3) ----
    "Webhook Details":            [r"PaymentWebhook", r"RefundWebhook", r"OutgoingWebhook"],
    "Outgoing Webhook Custom Headers": [r"updatebusinessprofilewebhookcustomheaders", r"webhookcustomheaders", r"webhook_custom_http_headers", r"custom webhook headers"],
    "Redirect Method":            [r"merchant redirect method", r"52-merchantredirectmethod", r"redirect_to_merchant_with_http_post"],
    "Product Type":               [r"merchant account product type", r"00003-producttype", r"product_type"],

    # ---- Customer / Mandate / Sync / Sav (B3) ----
    "Customer Management":        [r"CustomerCreate", r"CustomerList", r"DeletedCustomerPsync", r"CustomerFlow"],
    "Mandate Management":         [r"SingleuseMandate", r"MultiuseMandate", r"ListAndRevokeMandate", r"ZeroAuthMandate"],
    "Payment Sync":               [r"SyncPayment", r"PsyncFlow"],
    "Save Card Flow":             [r"SaveCard", r"SaveCardFlow"],
    "Off Session Payments":       [r"OffSession", r"ZeroAuthMandate"],

    # ---- Misc B3 ----
    "Eligibility Check":          [r"PaymentsEligibility", r"Eligibility"],
    "Authentication Service Eligibility":  [r"AuthenticationServiceEligibility", r"Authentication Service Eligibility"],
    "Eligibility Data Storage For Auth":   [r"AuthenticationServiceEligibility", r"eligibility.*storage", r"store_eligibility"],
    "Connector Agnostic MIT":     [r"ConnectorAgnosticNTID", r"ConnectorAgnosticMIT"],
    "External Vault":             [r"ExternalVault"],
    "External 3DS Authentication": [r"ExternalThreeDS", r"External3DS"],
    "Multiple Capture":           [r"MultipleCapture"],
    "Void/Cancel Payment":        [r"VoidPayment"],
    "Payment Link":               [r"PaymentLink"],
    "Payment Manual Update":      [r"ManualPaymentUpdate", r"Manual Payment Update", r"47-ManualPaymentUpdate"],
    "Forex/Currency Conversion":  [r"ForexRates", r"Forex Rates", r"forex_rates", r"Currency Conversion"],
    "Extended Card Info":         [r"ExtendedCardInfo", r"Extended Card Info", r"extended_card_info", r"51-ExtendedCardInfo"],
    "Extended Card BIN":          [r"ExtendedCardInfo", r"Extended Card BIN", r"extended_card_bin"],
    "Poll Config":                [r"PollConfig"],
     "PM Collect Link":            [r"paymentMethodCollect", r"Payment Method Collect Link", r"pm_collect_link"],
     "Refund Manual Update":       [r"RefundManualUpdate", r"Refund Manual Update"],
     "Payout Type":               [r"PayoutType", r"Payout Type", r"payout_type", r"00003-PayoutType"],
     "Payout Priority":           [r"PayoutPriority", r"Payout Priority", r"priority"],
     "Payout Auto Fulfill":       [r"AutoFulfill", r"Auto Fulfill", r"auto_fulfill", r"00003-AutoFulfill"],
     "Payout Entity Type":        [r"EntityType", r"Entity Type", r"entity_type", r"00007-EntityType"],
     "Payout Recurring":          [r"PayoutRecurring", r"Recurring Payout", r"recurring"],
     "Surcharge DSL":             [r"SurchargeDSL", r"Surcharge DSL"],
     "MIT With Limited Card Data": [r"MITWithLimitedCardData", r"MIT with Limited Card Data", r"mit-card-limited"],
     "PM Modular Service":         [r"ModularPmService", r"Modular PM Service", r"pm_modular"],
     "OIDC Authentication":        [r"Oidc"],
     "Iframe Redirection":         [r"Iframe"],
    "Health Check":               [r"HealthCheck"],
    "Card Testing Guard":         [r"CardTestingGuard"],
    "Payment Method Operations":  [r"PaymentMethodList", r"PaymentMethodCreate"],
    "SDK Client Token Generation": [r"SessionCall", r"session-call", r"SessionToken", r"ClientToken"],
    "Dispute Management":         [r"DisputeTests", r"\bDispute\b"],
    "FRM (Fraud Risk Management)": [r"FRM", r"FraudCheck"],

    # ---- B1 features that have dedicated spec files ----
    "Partner Merchant Identifier": [r"PartnerMerchantIdentifier"],
    "Connector Testing Data":      [r"ConnectorTestingData"],
    "Billing Descriptor":          [r"BillingDescriptor"],
    "Incremental Authorization":   [r"IncrementalAuth"],
    "Overcapture":                 [r"Overcapture"],
    "Network Transaction ID":      [r"NetworkTransactionId", r"NTID"],
    "L2/L3 Data Processing":       [r"L2L3Data", r"L2L3", r"LevelTwo", r"LevelThree"],
    "Step Up Authentication":      [r"StepUpAuth", r"step.up auth"],
    "Step Up Retry":               [r"StepUpRetr", r"step.up retry", r"step.up retries"],
    "Order Details":               [r"OrderDetails", r"order.details", r"52-orderdetails"],
    "Relay Operations":            [r"RelayOperations", r"relay.operations", r"52-relayoperations"],

    # ---- B3 features covered implicitly by StepUpAuth spec ----
    # 47-StepUpAuth.cy.js calls UpdateBusinessProfileTest with merchant_country_code
    # and merchant_category_code, exercising these business profile fields.
    # It also exercises the external 3DS authentication flow end-to-end
    # (authentication connector creation + 3DS authentication endpoint calls).
    "Merchant Category Code":      [r"StepUpAuth", r"step.up auth"],
    "Merchant Country Code":       [r"StepUpAuth", r"step.up auth"],
    "External 3DS Authentication": [r"StepUpAuth", r"step.up auth", r"ExternalThreeDS", r"External3DS"],

    # ---- B3 features whose spec files were not previously mapped ----
    "Use Billing As PM Billing":   [r"UseBillingAsPaymentMethodBilling", r"Use Billing As Payment Method Billing"],
    "Session Expiry":              [r"SessionExpiry", r"session_expiry"],
    "Card Issuer Management":      [r"CardIssuerManagement", r"Card Issuer Management"],
    "Feature Metadata":            [r"FeatureMetadata", r"feature_metadata"],
    "Connector Metadata":          [r"FeatureMetadata", r"feature_metadata"],   # covered by the same Feature Metadata spec
}


def parse_all_cypress_specs(spec_root):
    """
    Walk every .cy.js file under spec_root and return a dict keyed by
    full path with each value being:
        { 'filename': str, 'describes': [str], 'its': [str], 'haystack': str }
    `haystack` is the lowercase concatenation we run regex matches against.
    """
    out = {}
    if not os.path.isdir(spec_root):
        return out
    for dirpath, _, filenames in os.walk(spec_root):
        for fname in filenames:
            if not fname.endswith(".cy.js"):
                continue
            fpath = os.path.join(dirpath, fname)
            try:
                with open(fpath, encoding="utf-8") as f:
                    content = f.read()
            except OSError:
                continue
            describes = re.findall(r'describe\s*\(\s*["\'](.+?)["\']', content)
            its = re.findall(r'\bit\s*\(\s*["\'](.+?)["\']', content)
            haystack = (fname + " " + " ".join(describes) + " " + " ".join(its)).lower()
            out[fpath] = {
                "filename": fname,
                "describes": describes,
                "its": its,
                "haystack": haystack,
            }
    return out


def feature_covered_by_specs(feature, spec_index):
    """
    Return True if any spec's filename / describe / it title matches one of
    the FEATURE_SPEC_PATTERNS for `feature`. None if the feature has no
    pattern entry (caller falls back to existing detection).
    """
    patterns = FEATURE_SPEC_PATTERNS.get(feature)
    if not patterns:
        return None
    combined = re.compile("|".join(patterns), re.IGNORECASE)
    for spec in spec_index.values():
        if combined.search(spec["haystack"]):
            return True
    return False


def report_orphan_specs(spec_index):
    """Log any spec file that doesn't match any FEATURE_SPEC_PATTERNS entry."""
    if not spec_index:
        return
    all_patterns = re.compile(
        "|".join(p for plist in FEATURE_SPEC_PATTERNS.values() for p in plist),
        re.IGNORECASE,
    )
    orphans = []
    for path, spec in spec_index.items():
        if not all_patterns.search(spec["haystack"]):
            orphans.append(spec["filename"])
    if orphans:
        print(
            f"  [orphan-specs] {len(orphans)} spec files match no FEATURE_SPEC_PATTERNS entry:",
            file=sys.stderr,
        )
        for f in sorted(set(orphans))[:20]:
            print(f"    - {f}", file=sys.stderr)
        if len(set(orphans)) > 20:
            print(f"    … and {len(set(orphans)) - 20} more", file=sys.stderr)


def report_orphan_include_lists(include_lists):
    """
    Log any Utils.js INCLUDE list key that is not documented in
    UTILS_INCLUDE_FEATURE_MAP. These are new entries added by recent Cypress
    PRs that have not yet been mapped to a feature — coverage for those
    connectors will be silently wrong until an entry is added to the map.

    This is the analogue of report_orphan_specs() for INCLUDE lists.
    """
    # __EXCLUDE__ is an internal sentinel added by parse_cypress_configs()
    known = set(UTILS_INCLUDE_FEATURE_MAP.keys()) | {"__EXCLUDE__"}
    orphans = sorted(k for k in include_lists if k not in known)
    if orphans:
        print(
            f"\n  [ACTION REQUIRED] {len(orphans)} Utils.js INCLUDE list(s) are not in "
            f"UTILS_INCLUDE_FEATURE_MAP — cypress coverage for these connectors will NOT "
            f"be reflected until you add entries to the map:",
            file=sys.stderr,
        )
        for k in orphans:
            connectors = sorted(include_lists[k])
            print(f"    - {k}: {connectors}", file=sys.stderr)
        print(
            "  Add each key to UTILS_INCLUDE_FEATURE_MAP in extract_features.py "
            "(set value to feature name for B1, or None for B2/B3/structural).\n",
            file=sys.stderr,
        )


# ---- Bucket 3: Core features (static list - these don't change with connector code) ----
BUCKET_3_FEATURES = [
    ("Routing Algorithm", "Payment routing rules and algorithm", "POST /routing + GET /routing/{id}", "business_profile.rs:routing_algorithm", "covered", "Cypress specs: PriorityRouting VolumeBasedRouting RuleBasedRouting"),
    ("Dynamic Routing", "AI/ML-based dynamic routing algorithm", "POST /routing (dynamic type)", "business_profile.rs:dynamic_routing_algorithm", "not_covered", ""),
    ("FRM Routing Algorithm", "Fraud risk management routing rules", "POST /accounts/{merchant_id} (frm_routing_algorithm field)", "business_profile.rs:frm_routing_algorithm", "not_covered", ""),
    ("Payout Routing Algorithm", "Payout routing rules", "POST /routing (payout type)", "business_profile.rs:payout_routing_algorithm", "not_covered", ""),
    ("Default Fallback Routing", "Fallback routing when primary routing fails", "POST /business_profile (default_fallback_routing)", "business_profile.rs:default_fallback_routing", "not_covered", ""),
    ("3DS Decision Rule Algorithm", "Rules engine for 3DS authentication decisions", "POST /routing (3ds decision type)", "business_profile.rs:three_ds_decision_rule_algorithm", "not_covered", ""),
    ("External 3DS Authentication", "External 3DS authentication service integration", "POST /payments (request_external_three_ds_authentication)", "business_profile.rs:authentication_connector_details", "not_covered", ""),
    ("Authentication Service Eligibility", "Eligibility for authentication service", "POST /payments (read during 3DS auth eligibility check)", "configs table:authentication_service_eligible", "not_covered", ""),
    ("Eligibility Check", "Perform payment eligibility checks", "POST /payments/eligibility", "configs table:should_perform_eligibility", "covered", "Cypress spec 35-PaymentsEligibilityAPI"),
    ("Eligibility Data Storage For Auth", "Store eligibility check data for authentication", "POST /payments (read during UAS 3DS flow)", "configs table:should_store_eligibility_check_data_for_authentication", "not_covered", ""),
    ("3DS Routing Region UAS", "3DS routing region for Unified Authentication Service", "POST /payments (read during UAS 3DS routing)", "configs table:threeds_routing_region_uas", "not_covered", ""),
    ("PM Modular Service", "Payment method modular service toggle", "POST /payments/confirm + GET /payment_methods", "configs table:should_call_pm_modular_service", "not_covered", ""),
    ("Payment Link", "Generate hosted payment link for a payment", "POST /payments (payment_link=true) + GET /payment_link/{id}", "business_profile.rs:payment_link_config", "not_covered", ""),
    ("Payout Link", "Generate hosted payout link", "POST /payouts (payout_link config)", "business_profile.rs:payout_link_config", "not_covered", ""),
    ("PM Collect Link", "Payment method collection link", "GET /pm_collect/{id}", "merchant_account.rs:pm_collect_link_config", "not_covered", ""),
    ("Customer Management", "Customer CRUD operations", "POST/GET/PUT/DELETE /customers", "api_models/customers.rs", "covered", "Cypress spec 02-CustomerCreate 27-DeletedCustomerPsyncFlow 34-CustomerListTests"),
    ("Blocklist", "Block specific cards/fingerprints/card BINs", "POST/GET/DELETE /blocklist", "configs table:guard_blocklist_for", "covered", "Cypress spec 35-PaymentsEligibilityAPI"),
    ("PM Filters CGraph", "Payment method filter configuration graph", "GET /payment_methods (pm_filters_cgraph applied)", "configs table:pm_filters_cgraph", "covered", "Cypress spec 24/PaymentMethodList tests"),
    ("Conditional Routing DSL", "Conditional payment routing domain-specific language", "POST /routing (conditional config)", "configs table:dsl", "not_covered", ""),
    ("Surcharge DSL", "Surcharge decision logic rules", "POST /routing (surcharge config)", "configs table:surcharge_dsl", "not_covered", ""),
    ("Auto Retries", "Automatic payment retry on failure", "POST /payments (internal retry on failure)", "business_profile.rs:is_auto_retries_enabled", "covered", "Cypress spec 42-AutoRetries"),
    ("Clear PAN Retries", "Retry with clear PAN instead of network token", "POST /payments (internal retry strategy)", "business_profile.rs:is_clear_pan_retries_enabled", "not_covered", ""),
    ("Manual Retry", "Manual payment retry by merchant", "POST /payments/{id}/retry", "business_profile.rs:is_manual_retry_enabled", "covered", "Cypress spec 33-ManualRetry"),
    ("Requires CVV", "CVV requirement for saved card payments", "POST /payments (card_cvc required/optional)", "configs table:requires_cvv", "not_covered", ""),
    ("Implicit Customer Update", "Implicit customer record update behavior", "POST /payments (internal customer update)", "configs table:implicit_customer_update", "not_covered", ""),
    ("Client Session Validation", "Client session token validation", "POST /payments (client_secret validation)", "superposition:client_session_validation_enabled", "covered", "Cypress spec ClientSessionValidation (PR #12465)"),
    ("Payment Update Via Client Auth", "Allow payment updates via client authentication", "PUT /payments/{id} (client auth)", "configs table:payment_update_enabled_for_client_auth", "not_covered", ""),
    ("Raw PM Details Return", "Return raw payment method details in response", "GET /payment_methods (response format)", "configs table:should_return_raw_payment_method_details", "not_covered", ""),
    ("MIT With Limited Card Data", "Enable MIT with limited card data", "POST /payments (MIT with limited data)", "configs table:should_enable_mit_with_limited_card_data", "not_covered", ""),
    ("Extended Card BIN", "Extended card BIN lookup (8-digit)", "POST /payments (extended BIN in response)", "configs table:enable_extended_card_bin", "not_covered", ""),
    ("Extended Card Info", "Extended card information display", "POST /payments (extended info in response)", "business_profile.rs:is_extended_card_info_enabled", "not_covered", ""),
    ("Connector Agnostic MIT", "Connector-agnostic merchant-initiated transactions", "POST /payments (MIT across connectors)", "business_profile.rs:is_connector_agnostic_mit_enabled", "covered", "Cypress spec 25-ConnectorAgnosticNTID"),
    ("Use Billing As PM Billing", "Use payment billing address as payment method billing", "POST /payments (internal address mapping)", "business_profile.rs:use_billing_as_payment_method_billing", "not_covered", ""),
    ("Tax Connector", "External tax calculation connector", "POST /payments (internal tax calc)", "business_profile.rs:is_tax_connector_enabled", "not_covered", ""),
    ("Iframe Redirection", "Enable iframe-based redirection for hosted flows", "POST /payments (is_iframe_redirection_enabled)", "business_profile.rs:is_iframe_redirection_enabled", "not_covered", ""),
    ("External Vault", "External vault for payment method tokenization", "POST /payments (external vault flow)", "business_profile.rs:is_external_vault_enabled", "covered", "Cypress spec 40-ExternalVault"),
    ("CVV Collection During Payment", "Collect CVV during payment for saved cards", "POST /payments (CVV collection behavior)", "business_profile.rs:should_collect_cvv_during_payment", "not_covered", ""),
    ("Split Transactions Enabled", "Enable split transaction feature", "POST /business_profile (split_txns_enabled)", "business_profile.rs:split_txns_enabled", "not_covered", ""),
    ("Webhook Config Disabled Events", "Disable specific webhook events per connector", "POST /business_profile (webhook config)", "configs table:whconf_disabled_events", "not_covered", ""),
    ("Outgoing Webhook Custom Headers", "Custom HTTP headers for outgoing webhooks", "POST /business_profile (custom headers)", "business_profile.rs:outgoing_webhook_custom_http_headers", "covered", "Cypress spec 28-BusinessProfileConfigs (updateBusinessProfileWebhookCustomHeaders)"),
    ("Webhook Details", "Webhook URL and event configuration", "POST /business_profile (webhook_details)", "business_profile.rs:webhook_details", "covered", "Cypress specs 44-PaymentWebhook 45-RefundWebhook"),
    ("Card Testing Guard", "Anti-card-testing fraud detection", "POST /payments (internal fraud guard)", "business_profile.rs:card_testing_guard_config", "not_covered", ""),
    ("Payment Response Hash", "Sign payment response for integrity verification", "POST /payments (hash in response)", "business_profile.rs:enable_payment_response_hash", "covered", "Cypress spec 52-PaymentResponseHash (PR #12226)"),
    ("Redirect Method", "POST vs GET for merchant redirect", "POST /payments (redirect behavior)", "business_profile.rs:redirect_to_merchant_with_http_post", "covered", "Cypress spec 52-MerchantRedirectMethod (PR #12200)"),
    ("Session Expiry", "Client secret / session expiry time", "POST /payments (session timeout)", "business_profile.rs:session_expiry", "not_covered", ""),
    ("Reconciliation", "Payment reconciliation feature", "Recon API endpoints", "business_profile.rs:is_recon_enabled", "not_covered", ""),
    ("Sub-Merchants", "Sub-merchant management feature", "POST /merchant_account (sub_merchants_enabled)", "merchant_account.rs:sub_merchants_enabled", "not_covered", ""),
    ("Platform Account", "Platform/marketplace account type", "POST /organization + POST /merchant_account", "merchant_account.rs:is_platform_account", "covered", "Cypress Platform/ test suite"),
    ("Product Type", "Merchant product type (Payments/Payouts)", "POST /merchant_account (product_type)", "merchant_account.rs:product_type", "covered", "Cypress spec 00003-ProductType in Misc/ (PR #12029)"),
    ("Merchant Category Code", "Merchant category code (MCC)", "POST /business_profile (merchant_category_code)", "business_profile.rs:merchant_category_code", "not_covered", ""),
    ("Merchant Country Code", "Numeric merchant country code", "POST /business_profile (merchant_country_code)", "business_profile.rs:merchant_country_code", "not_covered", ""),
    ("Dispute Polling Interval", "Polling interval for dispute sync", "POST /business_profile (dispute_polling_interval)", "business_profile.rs:dispute_polling_interval", "not_covered", ""),
    ("Routing Result Source", "Which routing engine to use", "POST /payments (read during routing selection)", "configs table:routing_result_source", "not_covered", ""),
    ("Poll Config", "Payment status polling configuration", "GET /payments/{id} (read during redirect payment polling)", "configs table:poll_config", "not_covered", ""),
    ("Forex/Currency Conversion", "Currency conversion for payments", "GET /forex/rates", "settings.rs:forex_api", "not_covered", ""),
    ("Relay Operations", "Relay API for direct connector operations", "POST /relay (capture/refund/void/incremental_auth)", "api_models/relay.rs", "not_covered", ""),
    ("Subscription Management", "Subscription CRUD and lifecycle management", "POST/GET/PUT /subscriptions", "api_models/subscription.rs", "not_covered", ""),
    ("Dispute Management", "Dispute accept/defend/submit evidence", "POST /disputes/{id}/accept + POST /disputes/{id}/evidence", "api_models/disputes.rs", "not_covered", ""),
    ("FRM (Fraud Risk Management)", "Fraud check flows", "POST /payments (internal FRM check)", "merchant_connector_account.rs:frm_configs", "not_covered", ""),
    ("Network Tokenization Credentials", "Credentials for network tokenization", "POST /business_profile (network_tokenization_credentials)", "business_profile.rs:network_tokenization_credentials", "not_covered", ""),
    ("OIDC Authentication", "OpenID Connect authentication for users", "POST /oidc/token", "api_models/oidc.rs", "not_covered", ""),
    ("Card Issuer Management", "Card issuer CRUD operations", "POST/GET/PUT /card_issuers", "api_models/card_issuer.rs", "not_covered", ""),
    ("Payment Method Operations", "Payment method CRUD and listing", "GET /payment_methods + POST /payment_methods", "api_models/payment_methods.rs", "covered", "Cypress spec 24-PaymentMethods"),
    ("Organization Management", "Organization CRUD operations", "POST/GET/PUT /organization", "api_models/organization.rs", "covered", "Cypress spec 00-CoreFlows"),
    ("Merchant Account Management", "Merchant account CRUD operations", "POST/GET/PUT /accounts", "api_models/admin.rs", "covered", "Cypress spec 00-CoreFlows 01-AccountCreate"),
    ("Business Profile Management", "Business profile CRUD operations", "POST/GET/PUT /business_profile", "api_models/admin.rs", "covered", "Cypress spec 28-BusinessProfileConfigs"),
    ("MCA Management", "Merchant connector account CRUD operations", "POST/GET/PUT /account/{id}/connectors", "api_models/admin.rs", "covered", "Cypress spec 03-ConnectorCreate"),
    ("Browser Info Collection", "Browser information for 3DS and fraud detection", "POST /payments (browser_info in body)", "router_request_types.rs:browser_info", "covered", "Sent in all 3DS payment tests"),
    ("Order Details", "Order line item details", "POST /payments (order_details in body)", "api_models/payments.rs:order_details", "not_covered", ""),
    ("Customer Acceptance", "Customer acceptance for mandates/T&C", "POST /payments (customer_acceptance in body)", "api_models/payments.rs:customer_acceptance", "covered", "Tested in all mandate flows"),
    ("Off Session Payments", "Off-session (merchant-initiated) payment indicator", "POST /payments (off_session=true)", "api_models/payments.rs:off_session", "covered", "Tested in MIT mandate flows"),
    ("Feature Metadata", "Generic feature metadata on payments", "POST /payments (feature_metadata in body)", "api_models/payments.rs:feature_metadata", "not_covered", ""),
    ("Connector Metadata", "Connector-specific metadata from merchant", "POST /payments (connector_metadata in body)", "api_models/payments.rs:connector_metadata", "not_covered", ""),
    ("Multiple Capture", "Multiple partial captures on a single authorization", "POST /payments/{id}/capture (multiple times)", "router_request_types.rs:multiple_capture_data", "not_covered", ""),
    ("Payout Type", "Type of payout (card/bank/wallet)", "POST /payouts (payout_type in body)", "api_models/payouts.rs:payout_type", "covered", "Cypress payout specs"),
    ("Payout Priority", "Payout send priority", "POST /payouts (priority in body)", "api_models/payouts.rs:priority", "not_covered", ""),
    ("Payout Auto Fulfill", "Auto-fulfill payout without review", "POST /payouts (auto_fulfill=true)", "api_models/payouts.rs:auto_fulfill", "covered", "Cypress payout spec 00003"),
    ("Payout Entity Type", "Payout entity type classification", "POST /payouts (entity_type in body)", "api_models/payouts.rs:entity_type", "not_covered", ""),
    ("Payout Recurring", "Recurring payout indicator", "POST /payouts (recurring in body)", "api_models/payouts.rs:recurring", "not_covered", ""),
    ("Refund Type", "Refund type (instant/scheduled)", "POST /refunds (refund_type in body)", "api_models/refunds.rs:refund_type", "covered", "Cypress spec 54-RefundType (PR #12533)"),
    ("Mandate Management", "Mandate CRUD and revocation", "GET /mandates/{id} + POST /mandates/{id}/revoke", "api_models/mandates.rs", "covered", "Cypress spec 13-ListAndRevokeMandate"),
    ("Payment Manual Update", "Manual payment status update by admin", "PUT /payments/{id}/manual-update", "api_models/payments.rs", "not_covered", ""),
    ("Refund Manual Update", "Manual refund status update by admin", "PUT /refunds/{id}/manual-update", "api_models/refunds.rs", "not_covered", ""),
    ("Credentials Identifier Mapping", "MCA credentials identifier mapping", "POST /payments (read during MCA credential lookup)", "configs table:mcd_{merchant_id}", "not_covered", ""),
    ("Routing Evaluate", "Evaluate routing rules without creating payment", "POST /routing/evaluate", "api_models/routing.rs", "not_covered", ""),
    ("SDK Client Token Generation", "Generate SDK session/client tokens for frontend", "POST /payments/session_tokens", "hyperswitch_interfaces/api.rs", "covered", "Cypress spec 26-SessionCall"),
    ("Gateway Status Map (GSM)", "Map connector error codes to retry actions", "POST /payments (read during auto-retry flow)", "configs table:should_call_gsm", "not_covered", ""),
    ("Vault Tokenization Disable", "Disable vault tokenization for modular auth", "POST /payments (read during UAS modular auth flow)", "configs table:should_disable_vault_tokenization", "not_covered", ""),
    ("Delayed Session Response", "Delayed session response handling", "POST /payments/session_tokens (internal delay)", "settings.rs:delayed_session_response", "not_covered", ""),
    ("Connector API Version Override", "Override API version for a connector", "POST /payments (read when building connector request)", "configs table:connector_api_version", "not_covered", ""),
    ("Acquirer Config Map", "Acquirer-specific configurations", "POST /business_profile (acquirer_config_map)", "business_profile.rs:acquirer_config_map", "not_covered", ""),
    ("Payout Tracker Mapping", "Retry schedule mapping for payouts", "superposition config", "superposition:payout_tracker_mapping", "not_covered", ""),
    ("Process Tracker Mapping", "Retry schedule for sync operations", "GET /payments/{id} (read by payment_sync background workflow)", "configs table:pt_mapping", "not_covered", ""),
    ("Connector Onboarding Config", "Connector-specific onboarding configuration", "POST /account/{merchant_id}/connectors (read during connector onboarding)", "configs table:onboarding_{connector}", "not_covered", ""),
    ("Dynamic Fields", "SDK dynamic fields configuration", "GET /payment_methods (dynamic_fields)", "superposition:dynamic_fields", "covered", "Cypress spec 43-DynamicFields"),
    ("Save Card Flow", "Save card on-session and off-session", "POST /payments (setup_future_usage=on/off_session)", "api_models/payments.rs:setup_future_usage", "covered", "Cypress spec 14-SaveCardFlow"),
    ("Payment Sync", "Sync payment status with connector", "GET /payments/{id} (force_sync=true)", "api_models/payments.rs", "covered", "Cypress spec 08-SyncPayment"),
    ("Void/Cancel Payment", "Cancel/void an authorized payment", "POST /payments/{id}/cancel", "api_models/payments.rs", "covered", "Cypress spec 07-VoidPayment"),
    ("Health Check", "System health check endpoint", "GET /health", "misc", "covered", "Cypress spec Misc/00000-HealthCheck"),
]


def setup_db(db_path):
    """
    Create or refresh the schema. The `issues` table is the source of truth
    for the web app and is read+written by both the scheduler (auto-extracted
    fields) and the UI (human-edited fields: assignee, coverage_status,
    status, notes). Auto fields are upserted here without touching human
    fields; see upsert_issues().

    If an older issues schema is present (no `assignee` column), the table
    is dropped and recreated — auto fields are regenerated, human fields
    are then re-backfilled by merge_cypress_coverage.py.
    """
    conn = sqlite3.connect(db_path)
    conn.execute("PRAGMA journal_mode = WAL")
    conn.execute("PRAGMA foreign_keys = ON")

    existing = {r[1] for r in conn.execute("PRAGMA table_info(issues)").fetchall()}
    # `notes` is the marker for the redesigned schema (also has NOT NULL
    # defaults on connector/pm/pmt). If it's missing, the table is old —
    # drop and rebuild.
    if existing and "notes" not in existing:
        conn.execute("DROP TABLE issues")
        existing = set()

    if not existing:
        conn.execute("""
            CREATE TABLE issues (
                id                 INTEGER PRIMARY KEY AUTOINCREMENT,
                bucket             INTEGER NOT NULL CHECK (bucket IN (1,2,3)),
                connector          TEXT    NOT NULL DEFAULT '',
                pm                 TEXT    NOT NULL DEFAULT '',
                pmt                TEXT    NOT NULL DEFAULT '',
                feature            TEXT    NOT NULL,
                description        TEXT,
                hs_endpoint        TEXT,
                source             TEXT,
                cypress_status     TEXT    NOT NULL DEFAULT 'not_covered',
                prod_used          TEXT    NOT NULL DEFAULT 'unknown',
                prod_last_seen_at  TEXT,
                prod_checked_at    TEXT,
                assignee           TEXT,
                coverage_status    TEXT,
                status             TEXT    NOT NULL DEFAULT 'open'
                                       CHECK(status IN ('open','picked_up','covered')),
                notes              TEXT,
                created_at         TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at         TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(bucket, connector, pm, pmt, feature)
            )
        """)

    conn.execute("""
        CREATE TABLE IF NOT EXISTS pipeline_runs (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            started_at    TEXT NOT NULL,
            finished_at   TEXT,
            status        TEXT NOT NULL,
            log_path      TEXT,
            triggered_by  TEXT NOT NULL DEFAULT 'cron'
        )
    """)

    conn.commit()
    return conn


def upsert_issues(conn, rows):
    """
    Upsert auto-extracted rows. Human-edited columns (assignee,
    coverage_status, notes) are NEVER touched here — they only flow through
    merge_cypress_coverage.py or the web UI.

    Status rules:
      - cypress_status='covered'     → status='covered'
      - cypress_status changes away from covered → reset status to 'open'
      - otherwise preserve existing status ('open' or 'picked_up')
    """
    for row in rows:
        # Empty string instead of NULL so the UNIQUE key dedups properly.
        row["connector"] = row.get("connector") or ""
        row["pm"] = row.get("pm") or ""
        row["pmt"] = row.get("pmt") or ""
        row["initial_status"] = "covered" if row["cypress_status"] == "covered" else "open"

    conn.executemany("""
        INSERT INTO issues (bucket, connector, pm, pmt, feature, description,
                            hs_endpoint, source, cypress_status, status)
        VALUES (:bucket, :connector, :pm, :pmt, :feature, :description,
                :hs_endpoint, :source, :cypress_status, :initial_status)
        ON CONFLICT(bucket, connector, pm, pmt, feature) DO UPDATE SET
            description    = excluded.description,
            hs_endpoint    = excluded.hs_endpoint,
            source         = excluded.source,
            cypress_status = excluded.cypress_status,
            status = CASE
                WHEN excluded.cypress_status = 'covered'                                THEN 'covered'
                WHEN issues.status = 'covered' AND excluded.cypress_status != 'covered' THEN 'open'
                ELSE issues.status
            END,
            updated_at     = CURRENT_TIMESTAMP
    """, rows)
    conn.commit()


def main():
    print("Scanning codebase for features...", file=sys.stderr)

    db_conn = setup_db(DB_PATH)

    all_connectors = get_all_connectors()
    print(f"  Found {len(all_connectors)} connector modules", file=sys.stderr)

    # ---- Parse data sources ----
    default_impls = parse_default_implementations()
    print(f"  Parsed {len(default_impls)} default impl macros", file=sys.stderr)

    cypress_configs, include_lists = parse_cypress_configs()
    print(f"  Parsed {len(cypress_configs)} cypress connector configs", file=sys.stderr)
    report_orphan_include_lists(include_lists)

    # ---- Bucket 1 ----
    print("  Generating Bucket 1...", file=sys.stderr)
    b1_rows = set()

    # Method 1: Default impl exclusion (most reliable)
    for c, feat, desc, ep in detect_features_via_default_impls(all_connectors, default_impls):
        b1_rows.add((c, feat, desc, ep))

    # Method 2: Connector enum methods
    for c, feat, desc, ep in detect_connector_enum_features():
        b1_rows.add((c, feat, desc, ep))

    # Method 3: Transformer field usage
    for c, feat, desc, ep in detect_transformer_features():
        b1_rows.add((c, feat, desc, ep))

    # Method 4: Refund support (connector-level, sourced from SUPPORTED_PAYMENT_METHODS)
    for c, feat, desc, ep in detect_refund_support():
        b1_rows.add((c, feat, desc, ep))

    # Method 5: INCLUDE-list-derived B1 features (GSM/config-driven, not detectable from Rust code)
    # Each UTILS_INCLUDE_FEATURE_MAP entry that maps to a feature name AND whose include list
    # key ends with a known GSM/config-driven feature prefix is used to derive B1 rows.
    INCLUDE_DERIVED_B1 = {
        "STEP_UP_RETRY": (
            "Step Up Retry",
            "Payment that fails without 3DS is automatically retried with a 3DS step-up challenge",
            "POST /payments (GSM step_up_possible flow)",
        ),
        "PRE_AUTHENTICATION": (
            "Pre-Authentication Flow",
            "Initiate 3DS pre-authentication before the payment authorization request",
            "POST /payments/pre_auth (3DS pre-auth endpoint)",
        ),
    }
    for list_name, (feat, desc, ep) in INCLUDE_DERIVED_B1.items():
        for connector in include_lists.get(list_name, set()):
            b1_rows.add((connector, feat, desc, ep))

    # Filter excluded connectors and specific flow combinations
    b1_filtered = set()
    for c, feat, desc, ep in b1_rows:
        # Skip entirely excluded connectors
        if c.lower() in EXCLUDED_CONNECTORS:
            continue
        # Skip excluded connector+flow combinations
        if (c.lower(), feat) in EXCLUDED_FLOW_COMBINATIONS:
            continue
        b1_filtered.add((c, feat, desc, ep))

    # Deduplicate and sort
    b1_sorted = sorted(b1_filtered, key=lambda x: (x[1], x[0]))

    # Add cypress status
    b1_out = os.path.join(REPO_ROOT, "bucket_1_connector_features.csv")
    with open(b1_out, "w") as f:
        f.write("connector,feature,description,hs_endpoint,cypress_test_status\n")
        for connector, feature, description, endpoint in b1_sorted:
            cy = get_cypress_status_bucket1(connector, feature, cypress_configs, include_lists)
            # Quote description to handle commas
            f.write(f'{connector},{feature},"{description}",{endpoint},{cy}\n')

    print(f"  Bucket 1: {len(b1_sorted)} rows written to {b1_out}", file=sys.stderr)

    b1_db_rows = []
    for connector, feature, description, endpoint in b1_sorted:
        cy = get_cypress_status_bucket1(connector, feature, cypress_configs, include_lists)
        b1_db_rows.append({
            "bucket": 1, "connector": connector, "pm": None, "pmt": None,
            "feature": feature, "description": description,
            "hs_endpoint": endpoint, "source": None, "cypress_status": cy,
        })
    upsert_issues(db_conn, b1_db_rows)
    print(f"  Bucket 1: {len(b1_db_rows)} rows upserted to DB", file=sys.stderr)

    # ---- Bucket 2 ----
    print("  Generating Bucket 2...", file=sys.stderr)
    b2_entries = extract_bucket2()
    wallet_decrypt_rows = detect_wallet_decrypt_variants()

    b2_out = os.path.join(REPO_ROOT, "bucket_2_connector_pm_features.csv")
    b2_count = 0
    B2_DESCRIPTIONS = {
        "Payment": "Base payment authorization for this connector + PM + PMT combination",
        "Mandate": "Recurring/stored credential mandate setup for this connector + PM + PMT combination",
    }

    with open(b2_out, "w") as f:
        f.write("connector,payment_method,payment_method_type,feature,description,hs_endpoint,cypress_test_status\n")
        for connector, pm, pmt, mandates, refunds in b2_entries:
            # Payment - always supported
            cy = get_cypress_status_bucket2(connector, pm, pmt, "Payment", cypress_configs)
            f.write(f'{connector},{pm},{pmt},Payment,"{B2_DESCRIPTIONS["Payment"]}",POST /payments,{cy}\n')
            b2_count += 1

            # Mandate - only if Supported
            if mandates == "Supported":
                cy = get_cypress_status_bucket2(connector, pm, pmt, "Mandate", cypress_configs)
                f.write(f'{connector},{pm},{pmt},Mandate,"{B2_DESCRIPTIONS["Mandate"]}","POST /payments (setup_future_usage+mandate_data)",{cy}\n')
                b2_count += 1

        # Wallet decrypt flow variants (ApplePay/GooglePay/Paze decrypt)
        for connector, pm, pmt, feature, desc, endpoint in sorted(wallet_decrypt_rows):
            cy = get_cypress_status_bucket2(connector, pm, pmt, "Payment", cypress_configs)
            f.write(f'{connector},{pm},{pmt},"{feature}","{desc}","{endpoint}",{cy}\n')
            b2_count += 1

    print(f"  Bucket 2: {b2_count} rows written to {b2_out}", file=sys.stderr)

    b2_db_rows = []
    for connector, pm, pmt, mandates, refunds in b2_entries:
        cy = get_cypress_status_bucket2(connector, pm, pmt, "Payment", cypress_configs)
        b2_db_rows.append({
            "bucket": 2, "connector": connector, "pm": pm, "pmt": pmt,
            "feature": "Payment", "description": B2_DESCRIPTIONS["Payment"],
            "hs_endpoint": "POST /payments", "source": None, "cypress_status": cy,
        })
        if mandates == "Supported":
            cy = get_cypress_status_bucket2(connector, pm, pmt, "Mandate", cypress_configs)
            b2_db_rows.append({
                "bucket": 2, "connector": connector, "pm": pm, "pmt": pmt,
                "feature": "Mandate", "description": B2_DESCRIPTIONS["Mandate"],
                "hs_endpoint": "POST /payments (setup_future_usage+mandate_data)",
                "source": None, "cypress_status": cy,
            })
    for connector, pm, pmt, feature, desc, endpoint in sorted(wallet_decrypt_rows):
        cy = get_cypress_status_bucket2(connector, pm, pmt, "Payment", cypress_configs)
        b2_db_rows.append({
            "bucket": 2, "connector": connector, "pm": pm, "pmt": pmt,
            "feature": feature, "description": desc,
            "hs_endpoint": endpoint, "source": None, "cypress_status": cy,
        })
    upsert_issues(db_conn, b2_db_rows)
    print(f"  Bucket 2: {len(b2_db_rows)} rows upserted to DB", file=sys.stderr)

    # ---- Bucket 3 ----
    print("  Generating Bucket 3...", file=sys.stderr)

    # Spec-walk: dynamically determine cypress coverage for B3 features by
    # walking the cypress spec tree (incl. Routing/, etc.) and pattern-matching
    # spec filenames + describe/it titles against FEATURE_SPEC_PATTERNS.
    # Falls back to the hardcoded value in BUCKET_3_FEATURES when no pattern
    # is defined for that feature.
    spec_index = parse_all_cypress_specs(CYPRESS_SPEC_ROOT)
    print(f"  Indexed {len(spec_index)} cypress spec files", file=sys.stderr)

    b3_out = os.path.join(REPO_ROOT, "bucket_3_core_features.csv")
    b3_rows_resolved = []  # parallel list with cypress_status overridden
    overrides = 0
    
    # Filter out excluded Bucket 3 features
    b3_filtered = [row for row in BUCKET_3_FEATURES if row[0] not in EXCLUDED_FEATURES_BUCKET3]
    
    for row in b3_filtered:
        feature = row[0]
        hardcoded_status = row[4]
        detected = feature_covered_by_specs(feature, spec_index)
        if detected is None:
            resolved_status = hardcoded_status
        else:
            resolved_status = "covered" if detected else "not_covered"
            if resolved_status != hardcoded_status:
                overrides += 1
        b3_rows_resolved.append((row[0], row[1], row[2], row[3], resolved_status, row[5]))

    if overrides:
        print(f"  Bucket 3: spec-walk overrode hardcoded cypress_status on {overrides} features", file=sys.stderr)

    with open(b3_out, "w") as f:
        f.write("feature,description,hs_endpoint,source,cypress_test_status,notes\n")
        for row in b3_rows_resolved:
            escaped = []
            for field in row:
                if "," in str(field):
                    escaped.append(f'"{field}"')
                else:
                    escaped.append(str(field))
            f.write(",".join(escaped) + "\n")

    print(f"  Bucket 3: {len(b3_rows_resolved)} rows written to {b3_out}", file=sys.stderr)

    b3_db_rows = []
    for row in b3_rows_resolved:
        feature, description, hs_endpoint, source, cypress_status = row[0], row[1], row[2], row[3], row[4]
        b3_db_rows.append({
            "bucket": 3, "connector": None, "pm": None, "pmt": None,
            "feature": feature, "description": description,
            "hs_endpoint": hs_endpoint, "source": source, "cypress_status": cypress_status,
        })
    upsert_issues(db_conn, b3_db_rows)
    print(f"  Bucket 3: {len(b3_db_rows)} rows upserted to DB", file=sys.stderr)

    # Surface any spec files we don't recognize — these are gaps the team can
    # close by adding entries to FEATURE_SPEC_PATTERNS.
    report_orphan_specs(spec_index)
    # Surface any new Utils.js INCLUDE lists not yet mapped to features.
    report_orphan_include_lists(include_lists)

    db_conn.close()

    # ---- Overall Report CSV ----
    print("  Generating overall report...", file=sys.stderr)
    report_out = os.path.join(REPO_ROOT, "feature_extraction_report.csv")

    import csv as csv_mod
    from collections import Counter as _Counter

    def _cy_pct(counter):
        total = sum(counter.values())
        if total == 0:
            return "0%"
        return f"{round(counter.get('covered', 0) / total * 100)}%"

    # Bucket 1 stats
    b1_cy_c = _Counter()
    b1_feat_set, b1_conn_set = set(), set()
    with open(b1_out) as f:
        for row in csv_mod.DictReader(f):
            b1_cy_c[row["cypress_test_status"]] += 1
            b1_feat_set.add(row["feature"])
            b1_conn_set.add(row["connector"])

    # Bucket 2 stats
    b2_cy_c = _Counter()
    b2_conn_set, b2_pm_set, b2_pmt_set, b2_pm_pmt_set = set(), set(), set(), set()
    with open(b2_out) as f:
        for row in csv_mod.DictReader(f):
            b2_cy_c[row["cypress_test_status"]] += 1
            b2_conn_set.add(row["connector"])
            b2_pm_set.add(row["payment_method"])
            b2_pmt_set.add(row["payment_method_type"])
            b2_pm_pmt_set.add((row["payment_method"], row["payment_method_type"]))

    # Bucket 3 stats
    b3_cy_c = _Counter()
    with open(b3_out) as f:
        for row in csv_mod.DictReader(f):
            b3_cy_c[row["cypress_test_status"]] += 1

    b1_total = sum(b1_cy_c.values())
    b2_total = sum(b2_cy_c.values())
    b3_total = sum(b3_cy_c.values())
    grand_total = b1_total + b2_total + b3_total
    grand_cy_c = _Counter({
        "covered": b1_cy_c["covered"] + b2_cy_c["covered"] + b3_cy_c["covered"],
        "not_covered": b1_cy_c["not_covered"] + b2_cy_c["not_covered"] + b3_cy_c["not_covered"],
        "no_cypress_config": b1_cy_c["no_cypress_config"] + b2_cy_c.get("no_cypress_config", 0) + b3_cy_c.get("no_cypress_config", 0),
    })

    with open(report_out, "w", newline="") as f:
        w = csv_mod.writer(f)
        w.writerow([
            "bucket", "description",
            "total_rows", "unique_features", "unique_connectors",
            "unique_payment_methods", "unique_pm_types",
            "cypress_covered", "cypress_not_covered", "cypress_no_config",
            "cypress_coverage_pct",
        ])
        w.writerow([
            "Bucket 1", "Connector × Feature (payload-level, PM-agnostic)",
            b1_total, len(b1_feat_set), len(b1_conn_set),
            "", "",
            b1_cy_c["covered"], b1_cy_c["not_covered"], b1_cy_c["no_cypress_config"],
            _cy_pct(b1_cy_c),
        ])
        w.writerow([
            "Bucket 2", "Connector × PM × PMT × Feature",
            b2_total, len(b2_pm_pmt_set), len(b2_conn_set),
            len(b2_pm_set), len(b2_pmt_set),
            b2_cy_c["covered"], b2_cy_c["not_covered"], b2_cy_c.get("no_cypress_config", 0),
            _cy_pct(b2_cy_c),
        ])
        w.writerow([
            "Bucket 3", "Core features (connector-agnostic)",
            b3_total, b3_total, "",
            "", "",
            b3_cy_c["covered"], b3_cy_c["not_covered"], "",
            _cy_pct(b3_cy_c),
        ])
        w.writerow([
            "TOTAL", "All buckets combined",
            grand_total, "", "",
            "", "",
            grand_cy_c["covered"], grand_cy_c["not_covered"], grand_cy_c["no_cypress_config"],
            _cy_pct(grand_cy_c),
        ])

    print(f"  Report: {report_out}", file=sys.stderr)

    print("\nDone. Generated:", file=sys.stderr)
    print(f"  {b1_out}", file=sys.stderr)
    print(f"  {b2_out}", file=sys.stderr)
    print(f"  {b3_out}", file=sys.stderr)
    print(f"  {report_out}", file=sys.stderr)
    print(f"  {DB_PATH}", file=sys.stderr)


if __name__ == "__main__":
    main()
