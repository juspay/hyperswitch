import base64
import json
import unittest
from urllib.parse import urlencode

from callback_urls import CallbackUrlStore, callback_urls


TOKEN = "d9dc53cc-d1a1-4f57-9077-d20c2d614daa"
NEW_TOKEN = "d9dc53cc-d1a1-4f57-9077-d20c2d614dab"
CALLBACK = f"http://localhost:8080/payments/pay_1/merchant_1/redirect/token/{TOKEN}/complete/paybox"
NOTIFY = CALLBACK.replace(TOKEN, NEW_TOKEN)


class CallbackUrlTests(unittest.TestCase):
    def test_redsys_encoded_3ds_callback_excludes_webhooks(self):
        callback = CALLBACK.replace("/paybox", "/redsys")
        parameters = {
            "DS_MERCHANT_EMV3DS": {"notificationURL": callback, "notify_url": NOTIFY},
            "DS_MERCHANT_MERCHANTURL": NOTIFY,
        }
        payload = {
            "Ds_MerchantParameters": base64.b64encode(
                json.dumps(parameters).encode()
            ).decode(),
            "Ds_SignatureVersion": "HMAC_SHA256_V1",
            "Ds_Signature": "signature",
        }
        for body in [json.dumps(payload), urlencode(payload)]:
            with self.subTest(body=body):
                store = CallbackUrlStore()
                store.capture("https://redsys.example/authorize", body, "confirm")
                self.assertEqual(store.for_payment("pay_1", "merchant_1"), [callback])

    def test_redsys_malformed_or_non_callback_parameters_are_ignored(self):
        for parameters in [
            "not base64!",
            "",
            None,
            123,
            base64.b64encode(b"not json").decode(),
            base64.b64encode(b"\xff").decode(),
            *[
                base64.b64encode(json.dumps(value).encode()).decode()
                for value in [
                    [],
                    {"DS_MERCHANT_EMV3DS": None},
                    {"DS_MERCHANT_EMV3DS": {"notificationURL": "https://merchant.example"}},
                    {"DS_MERCHANT_MERCHANTURL": NOTIFY},
                ]
            ],
        ]:
            with self.subTest(parameters=parameters):
                self.assertEqual(
                    list(callback_urls({"Ds_MerchantParameters": parameters})), []
                )

    def test_connector_payload_formats_exclude_notification_tokens(self):
        payload = {"URLRetour": CALLBACK, "URLHttpDirect": NOTIFY}
        for body in [
            json.dumps(payload),
            urlencode(payload),
            f"<request><URLRetour>{CALLBACK}</URLRetour>"
            f"<URLHttpDirect>{NOTIFY}</URLHttpDirect></request>",
            json.dumps({"request": [payload]}),
            json.dumps({"accept_url": CALLBACK, "notify_url": NOTIFY}),
            json.dumps({"return_url": CALLBACK, "notificationURL": NOTIFY}),
        ]:
            with self.subTest(body=body):
                self.assertEqual(list(callback_urls(body)), [CALLBACK])

    def test_url_query_and_json_escaped_urls(self):
        for value in [
            "https://connector.example/authorize?" + urlencode({"return_url": CALLBACK}),
            json.dumps({"return_url": CALLBACK}).replace("/", r"\/"),
        ]:
            self.assertEqual(list(callback_urls(value)), [CALLBACK])

    def test_legacy_and_non_callback_urls_are_not_replayed(self):
        for value in [
            "http://localhost:8080/payments/pay_1/merchant_1/redirect/complete/paybox",
            CALLBACK.replace("/complete/", "/start/"),
            "https://merchant.example/return",
        ]:
            self.assertEqual(list(callback_urls(value)), [])

    def test_current_tokens_are_scoped_to_payment_and_retries(self):
        store = CallbackUrlStore()
        store.capture("https://connector.example", json.dumps({"return_url": CALLBACK}), "confirm")
        self.assertEqual(store.for_payment("pay_1", "merchant_1"), [CALLBACK])
        self.assertEqual(store.for_payment("pay_2", "merchant_1"), [])
        self.assertEqual(store.for_payment("pay_1", "merchant_2"), [])

        # Cypress often confirms in one test and handles the redirect in another.
        store.reset_for_hash("redirect")
        self.assertEqual(store.for_payment("pay_1", "merchant_1"), [CALLBACK])
        store.capture("https://connector.example", json.dumps({"return_url": NOTIFY}), "confirm")
        self.assertEqual(store.for_payment("pay_1", "merchant_1"), [NOTIFY])
        store.reset_for_hash("confirm")
        self.assertEqual(store.for_payment("pay_1", "merchant_1"), [])


if __name__ == "__main__":
    unittest.main()
