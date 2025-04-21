from http import HTTPStatus
from typing import Any, Optional, Union

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.payments_confirm_request import PaymentsConfirmRequest
from ...types import Response


def _get_kwargs(
    payment_id: str,
    *,
    body: PaymentsConfirmRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/payments/{payment_id}/confirm",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Optional[Any]:
    if response.status_code == 400:
        return None
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(*, client: Union[AuthenticatedClient, Client], response: httpx.Response) -> Response[Any]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    payment_id: str,
    *,
    client: AuthenticatedClient,
    body: PaymentsConfirmRequest,
) -> Response[Any]:
    """Payments - Confirm

     **Use this API to confirm the payment and forward the payment to the payment processor.**

    Alternatively you can confirm the payment within the *Payments/Create* API by setting
    `confirm=true`. After confirmation, the payment could either:

    1. fail with `failed` status or

    2. transition to a `requires_customer_action` status with a `next_action` block or

    3. succeed with either `succeeded` in case of automatic capture or `requires_capture` in case of
    manual capture

    Args:
        payment_id (str):
        body (PaymentsConfirmRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        payment_id=payment_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    payment_id: str,
    *,
    client: AuthenticatedClient,
    body: PaymentsConfirmRequest,
) -> Response[Any]:
    """Payments - Confirm

     **Use this API to confirm the payment and forward the payment to the payment processor.**

    Alternatively you can confirm the payment within the *Payments/Create* API by setting
    `confirm=true`. After confirmation, the payment could either:

    1. fail with `failed` status or

    2. transition to a `requires_customer_action` status with a `next_action` block or

    3. succeed with either `succeeded` in case of automatic capture or `requires_capture` in case of
    manual capture

    Args:
        payment_id (str):
        body (PaymentsConfirmRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        payment_id=payment_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)
