from http import HTTPStatus
from typing import Any, Optional, Union

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.payments_create_request import PaymentsCreateRequest
from ...types import Response


def _get_kwargs(
    *,
    body: PaymentsCreateRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": "/payments",
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
    *,
    client: AuthenticatedClient,
    body: PaymentsCreateRequest,
) -> Response[Any]:
    """Payments - Create

     **Creates a payment object when amount and currency are passed.**

    This API is also used to create a mandate by passing the `mandate_object`.

    Depending on the user journey you wish to achieve, you may opt to complete all the steps in a single
    request **by attaching a payment method, setting `confirm=true` and `capture_method = automatic`**
    in the *Payments/Create API* request.

    Otherwise, To completely process a payment you will have to **create a payment, attach a payment
    method, confirm and capture funds**. For that you could use the following sequence of API requests -

    1. Payments - Create

    2. Payments - Update

    3. Payments - Confirm

    4. Payments - Capture.

    You will require the 'API - Key' from the Hyperswitch dashboard to make the first call, and use the
    'client secret' returned in this API along with your 'publishable key' to make subsequent API calls
    from your client.

    This page lists the various combinations in which the Payments - Create API can be used and the
    details about the various fields in the requests and responses.

    Args:
        body (PaymentsCreateRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


async def asyncio_detailed(
    *,
    client: AuthenticatedClient,
    body: PaymentsCreateRequest,
) -> Response[Any]:
    """Payments - Create

     **Creates a payment object when amount and currency are passed.**

    This API is also used to create a mandate by passing the `mandate_object`.

    Depending on the user journey you wish to achieve, you may opt to complete all the steps in a single
    request **by attaching a payment method, setting `confirm=true` and `capture_method = automatic`**
    in the *Payments/Create API* request.

    Otherwise, To completely process a payment you will have to **create a payment, attach a payment
    method, confirm and capture funds**. For that you could use the following sequence of API requests -

    1. Payments - Create

    2. Payments - Update

    3. Payments - Confirm

    4. Payments - Capture.

    You will require the 'API - Key' from the Hyperswitch dashboard to make the first call, and use the
    'client secret' returned in this API along with your 'publishable key' to make subsequent API calls
    from your client.

    This page lists the various combinations in which the Payments - Create API can be used and the
    details about the various fields in the requests and responses.

    Args:
        body (PaymentsCreateRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Any]
    """

    kwargs = _get_kwargs(
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)
