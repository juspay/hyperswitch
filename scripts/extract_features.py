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
API_RS = os.path.join(REPO_ROOT, "crates/hyperswitch_interfaces/src/api.rs")
DB_PATH = os.path.join(REPO_ROOT, "features.db")


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
    "default_imp_for_pre_processing_steps": {
        "feature": "Preprocessing Flow",
        "description": "Connector-specific setup step before authorization (e.g. session creation, device fingerprinting, BNPL eligibility check)",
        "endpoint": "POST /payments (internal preprocessing before auth)",
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

    if not os.path.exists(CYPRESS_DIR):
        return configs

    pm_categories = [
        "card_pm", "bank_transfer_pm", "bank_redirect_pm", "wallet_pm",
        "upi_pm", "crypto_pm", "reward_pm", "pay_later_pm", "bank_debit_pm",
        "voucher_pm", "real_time_pm", "gift_card_pm", "open_banking_pm",
        "mobile_payment_pm", "card_redirect_pm",
    ]

    for fname in os.listdir(CYPRESS_DIR):
        if not fname.endswith(".js") or fname in skip:
            continue
        connector = fname.replace(".js", "").lower()
        fpath = os.path.join(CYPRESS_DIR, fname)
        with open(fpath) as f:
            content = f.read()

        pm_types = set()
        for cat in pm_categories:
            if cat in content:
                pm_types.add(cat)

        features = set()
        if "Refund" in content:
            features.add("Refund")
        if "Void" in content:
            features.add("Void")
        if "Mandate" in content:
            features.add("Mandate")
        if "SaveCard" in content:
            features.add("SaveCard")

        configs[connector] = {"pm_types": pm_types, "features": features}

    # Parse INCLUDE lists from Utils.js
    include_lists = {}
    if os.path.exists(CYPRESS_UTILS):
        with open(CYPRESS_UTILS) as f:
            utils_content = f.read()
        # Find INCLUDE section
        inc_match = re.search(r'INCLUDE:\s*\{(.*?)\}', utils_content, re.DOTALL)
        if inc_match:
            inc_body = inc_match.group(1)
            for list_match in re.finditer(
                r'(\w+):\s*\[(.*?)\]', inc_body, re.DOTALL
            ):
                name = list_match.group(1)
                connectors = re.findall(r'"(\w+)"', list_match.group(2))
                include_lists[name] = set(c.lower() for c in connectors)

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
    "RealTimePayment": "real_time_pm",
    "GiftCard": "gift_card_pm",
    "OpenBanking": "open_banking_pm",
    "MobilePayment": "mobile_payment_pm",
    "CardRedirect": "card_redirect_pm",
    "NetworkToken": "card_pm",
}


def get_cypress_status_bucket1(connector, feature, cypress_configs, include_lists):
    """Determine cypress coverage for a Bucket 1 (connector, feature) pair."""
    c = connector.lower()
    if c not in cypress_configs:
        return "no_cypress_config"

    # Check specific include lists
    feature_include_map = {
        "Incremental Authorization": "INCREMENTAL_AUTH",
        "Overcapture": "OVERCAPTURE",
        "Installments": "CARD_INSTALLMENTS",
    }
    if feature in feature_include_map:
        list_name = feature_include_map[feature]
        if list_name in include_lists and c in include_lists[list_name]:
            return "covered"
        return "not_covered"

    # Refund - check if connector cypress config has Refund tests
    if feature == "Refund":
        cfg = cypress_configs.get(c, {})
        return "covered" if "Refund" in cfg.get("features", set()) else "not_covered"

    # Network Transaction ID - check NTID proxy tests
    if feature == "Network Transaction ID":
        if "MANDATES_USING_NTID_PROXY" in include_lists and c in include_lists["MANDATES_USING_NTID_PROXY"]:
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

    if pm in ("Card", "NetworkToken"):
        if feature == "Payment":
            return "covered"
        if feature == "Refund":
            return "covered" if "Refund" in cfg["features"] else "not_covered"
        if feature == "Mandate":
            return "covered" if "Mandate" in cfg["features"] else "not_covered"

    if feature == "Payment":
        return "covered"

    return "not_covered"


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
    ("Client Session Validation", "Client session token validation", "POST /payments (client_secret validation)", "superposition:client_session_validation_enabled", "not_covered", ""),
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
    ("Outgoing Webhook Custom Headers", "Custom HTTP headers for outgoing webhooks", "POST /business_profile (custom headers)", "business_profile.rs:outgoing_webhook_custom_http_headers", "not_covered", ""),
    ("Webhook Details", "Webhook URL and event configuration", "POST /business_profile (webhook_details)", "business_profile.rs:webhook_details", "covered", "Cypress specs 44-PaymentWebhook 45-RefundWebhook"),
    ("Card Testing Guard", "Anti-card-testing fraud detection", "POST /payments (internal fraud guard)", "business_profile.rs:card_testing_guard_config", "not_covered", ""),
    ("Payment Response Hash", "Sign payment response for integrity verification", "POST /payments (hash in response)", "business_profile.rs:enable_payment_response_hash", "not_covered", ""),
    ("Redirect Method", "POST vs GET for merchant redirect", "POST /payments (redirect behavior)", "business_profile.rs:redirect_to_merchant_with_http_post", "not_covered", ""),
    ("Session Expiry", "Client secret / session expiry time", "POST /payments (session timeout)", "business_profile.rs:session_expiry", "not_covered", ""),
    ("Reconciliation", "Payment reconciliation feature", "Recon API endpoints", "business_profile.rs:is_recon_enabled", "not_covered", ""),
    ("Sub-Merchants", "Sub-merchant management feature", "POST /merchant_account (sub_merchants_enabled)", "merchant_account.rs:sub_merchants_enabled", "not_covered", ""),
    ("Platform Account", "Platform/marketplace account type", "POST /organization + POST /merchant_account", "merchant_account.rs:is_platform_account", "covered", "Cypress Platform/ test suite"),
    ("Product Type", "Merchant product type (Payments/Payouts)", "POST /merchant_account (product_type)", "merchant_account.rs:product_type", "not_covered", ""),
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
    ("Refund Type", "Refund type (instant/scheduled)", "POST /refunds (refund_type in body)", "api_models/refunds.rs:refund_type", "not_covered", ""),
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
    conn = sqlite3.connect(db_path)
    conn.execute("""
        CREATE TABLE IF NOT EXISTS issues (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            bucket         INTEGER NOT NULL,
            connector      TEXT,
            pm             TEXT,
            pmt            TEXT,
            feature        TEXT NOT NULL,
            description    TEXT,
            hs_endpoint    TEXT,
            source         TEXT,
            cypress_status TEXT,
            status         TEXT NOT NULL DEFAULT 'open'
                           CHECK(status IN ('open', 'picked_up', 'covered')),
            updated_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(bucket, connector, pm, pmt, feature)
        )
    """)
    conn.commit()
    return conn


def upsert_issues(conn, rows):
    """
    rows: list of dicts with keys:
      bucket, connector, pm, pmt, feature, description, hs_endpoint, source, cypress_status

    Status rules:
      - cypress_status='covered'     → status='covered' (always, cypress is source of truth)
      - cypress_status changes away from covered → reset status to 'open'
      - otherwise preserve existing status ('open' or 'picked_up')
    """
    for row in rows:
        row['initial_status'] = 'covered' if row['cypress_status'] == 'covered' else 'open'

    conn.executemany("""
        INSERT INTO issues (bucket, connector, pm, pmt, feature, description, hs_endpoint, source, cypress_status, status)
        VALUES (:bucket, :connector, :pm, :pmt, :feature, :description, :hs_endpoint, :source, :cypress_status, :initial_status)
        ON CONFLICT(bucket, connector, pm, pmt, feature) DO UPDATE SET
            description    = excluded.description,
            hs_endpoint    = excluded.hs_endpoint,
            source         = excluded.source,
            cypress_status = excluded.cypress_status,
            status = CASE
                WHEN excluded.cypress_status = 'covered'                           THEN 'covered'
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

    # Deduplicate and sort
    b1_sorted = sorted(b1_rows, key=lambda x: (x[1], x[0]))

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
    b3_out = os.path.join(REPO_ROOT, "bucket_3_core_features.csv")
    with open(b3_out, "w") as f:
        f.write("feature,description,hs_endpoint,source,cypress_test_status,notes\n")
        for row in BUCKET_3_FEATURES:
            # Escape commas in fields
            escaped = []
            for field in row:
                if "," in str(field):
                    escaped.append(f'"{field}"')
                else:
                    escaped.append(str(field))
            f.write(",".join(escaped) + "\n")

    print(f"  Bucket 3: {len(BUCKET_3_FEATURES)} rows written to {b3_out}", file=sys.stderr)

    b3_db_rows = []
    for row in BUCKET_3_FEATURES:
        feature, description, hs_endpoint, source, cypress_status = row[0], row[1], row[2], row[3], row[4]
        b3_db_rows.append({
            "bucket": 3, "connector": None, "pm": None, "pmt": None,
            "feature": feature, "description": description,
            "hs_endpoint": hs_endpoint, "source": source, "cypress_status": cypress_status,
        })
    upsert_issues(db_conn, b3_db_rows)
    print(f"  Bucket 3: {len(b3_db_rows)} rows upserted to DB", file=sys.stderr)

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
