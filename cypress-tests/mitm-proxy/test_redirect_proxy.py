import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import redirect_proxy


TOKEN = "d9dc53cc-d1a1-4f57-9077-d20c2d614daa"


class RedirectPathTests(unittest.TestCase):
    def test_redirect_detection_supports_legacy_and_token_paths(self):
        for action in ["response", "complete"]:
            for tail in ["stripe", "stripe/creds_1"]:
                for path in [
                    f"/payments/pay_1/merchant_1/redirect/{action}/{tail}",
                    f"/payments/pay_1/merchant_1/redirect/{action}/{tail}/callback/{TOKEN}",
                    f"/payments/pay_1/merchant_1/redirect/token/{TOKEN}/{action}/{tail}",
                ]:
                    with self.subTest(path=path):
                        self.assertTrue(redirect_proxy._is_redirect_complete(path))
                        self.assertEqual(redirect_proxy._extract_payment_id(path), "pay_1")
                        self.assertEqual(
                            redirect_proxy._extract_redirect_segment(path),
                            f"redirect/{action}/{tail}",
                        )
        for action in ["start", "authorize"]:
            self.assertFalse(redirect_proxy._is_redirect_complete(
                f"/payments/pay_1/merchant_1/redirect/token/{TOKEN}/{action}/stripe"
            ))

    def test_saved_redirect_body_keeps_connector_data_without_the_token(self):
        for method, body, content_type in [
            ("GET", b"", ""),
            ("POST", b"status=success", "application/x-www-form-urlencoded"),
        ]:
            data = redirect_proxy._build_redirect_data(
                method,
                f"/payments/pay_1/merchant_1/redirect/token/{TOKEN}/complete/paybox/creds_1",
                "result=ok",
                body,
                content_type,
            )
            self.assertEqual(data["__redirect_segment"], "redirect/complete/paybox/creds_1")
            self.assertNotIn(TOKEN, json.dumps(data))
            if method == "GET":
                self.assertEqual(data["__query"], {"result": "ok"})
            else:
                self.assertEqual(data["__body"], {"status": "success"})

    def test_capture_directory_uses_connector_even_with_credentials(self):
        data = {"__redirect_segment": "redirect/complete/paybox/creds_1"}
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(redirect_proxy, "CAPTURE_DIR", directory):
                with contextlib.redirect_stdout(io.StringIO()):
                    redirect_proxy._write_to_captures(data, "redirect.json")
            capture = Path(directory) / "paybox" / "Payment" / "redirect-bodies" / "redirect.json"
            self.assertEqual(json.loads(capture.read_text()), data)


if __name__ == "__main__":
    unittest.main()
