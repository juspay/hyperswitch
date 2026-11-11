"""Keep callback URLs from the current replay run, never from a cassette."""

import base64
import html
import json
import re
import threading
from urllib.parse import parse_qsl, unquote, urlsplit
from xml.etree import ElementTree


_URL = re.compile(r"https?://[^\s\"'<>\\]+")
_CALLBACK = re.compile(
    r"/payments/([^/]+)/([^/]+)/redirect/token/"
    r"[0-9a-fA-F-]{36}/(?:response|complete)/[^/]+(?:/[^/]+)?$"
)
_NOTIFICATION = re.compile(r"notify|notification|webhook|http_?direct", re.I)


def _redsys_callback_urls(value):
    try:
        parameters = json.loads(base64.b64decode(value, validate=True))
    except (ValueError, TypeError):
        return
    if isinstance(parameters, dict):
        emv3ds = parameters.get("DS_MERCHANT_EMV3DS")
        if isinstance(emv3ds, dict):
            # Redsys uses notificationURL for the browser's 3DS completion,
            # distinct from its DS_MERCHANT_MERCHANTURL webhook.
            yield from callback_urls(emv3ds.get("notificationURL"))


def callback_urls(value):
    """Read JSON, form and XML callback fields, excluding notification URLs."""
    if isinstance(value, dict):
        for key, item in value.items():
            if key == "Ds_MerchantParameters":
                yield from _redsys_callback_urls(item)
            elif not _NOTIFICATION.search(key):
                yield from callback_urls(item)
        return
    if isinstance(value, list):
        for item in value:
            yield from callback_urls(item)
        return
    if not isinstance(value, str):
        return

    value = value.strip()
    if value.startswith(("http://", "https://")):
        parts = urlsplit(html.unescape(value))
        if _CALLBACK.fullmatch(parts.path):
            yield html.unescape(value)
        for key, item in parse_qsl(parts.query):
            yield from callback_urls({key: item})
        return
    try:
        parsed = json.loads(value)
    except (ValueError, TypeError):
        parsed = None
    if isinstance(parsed, (dict, list)):
        yield from callback_urls(parsed)
        return
    if value.startswith("<"):
        try:
            root = ElementTree.fromstring(value)
        except ElementTree.ParseError:
            pass
        else:
            def visit(element):
                if not _NOTIFICATION.search(element.tag):
                    yield from callback_urls(element.text)
                    for child in element:
                        yield from visit(child)
            yield from visit(root)
            return
    if "=" in value:
        for key, item in parse_qsl(value):
            yield from callback_urls({key: item})
        return

    # Connector payloads can nest percent-encoded URLs (e.g. return_url).
    for _ in range(3):
        decoded = unquote(value)
        if decoded == value:
            break
        value = decoded
    value = html.unescape(value).replace(r"\/", "/")
    for match in _URL.finditer(value):
        url = match.group()
        if _CALLBACK.fullmatch(urlsplit(url).path):
            yield url


class CallbackUrlStore:
    def __init__(self):
        self._urls = {}
        self._lock = threading.Lock()

    def capture(self, request_url, request_body, test_hash):
        urls = set(callback_urls(request_url)) | set(callback_urls(request_body))
        with self._lock:
            for url in urls:
                # Later connector calls may issue a new token for the same route.
                path = re.sub(r"/token/[^/]+/", "/", urlsplit(url).path)
                self._urls[path] = (url, test_hash)

    def for_payment(self, payment_id, merchant_id):
        with self._lock:
            return [
                url for url, _ in self._urls.values()
                if _CALLBACK.fullmatch(urlsplit(url).path).groups() == (payment_id, merchant_id)
            ]

    def reset_for_hash(self, test_hash):
        with self._lock:
            # Confirm and redirect can be separate tests; keep other tests' URLs.
            self._urls = {
                path: entry for path, entry in self._urls.items()
                if entry[1] != test_hash
            }
