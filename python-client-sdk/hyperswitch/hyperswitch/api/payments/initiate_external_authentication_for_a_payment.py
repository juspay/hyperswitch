from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.payments_external_authentication_request import PaymentsExternalAuthenticationRequest
from ...models.payments_external_authentication_response import PaymentsExternalAuthenticationResponse
from ...types import Response


def _get_kwargs(
    payment_id: str,
    *,
    body: PaymentsExternalAuthenticationRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/payments/{payment_id}/3ds/authentication",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, PaymentsExternalAuthenticationResponse]]:
    if response.status_code == 200:
        response_200 = PaymentsExternalAuthenticationResponse.from_dict(response.json())

        return response_200
    if response.status_code == 400:
        response_400 = cast(Any, None)
        return response_400
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, PaymentsExternalAuthenticationResponse]]:
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
    body: PaymentsExternalAuthenticationRequest,
) -> Response[Union[Any, PaymentsExternalAuthenticationResponse]]:
    """Payments - External 3DS Authentication

     External 3DS Authentication is performed and returns the AuthenticationResponse

    Args:
        payment_id (str):
        body (PaymentsExternalAuthenticationRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PaymentsExternalAuthenticationResponse]]
    """

    kwargs = _get_kwargs(
        payment_id=payment_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    payment_id: str,
    *,
    client: AuthenticatedClient,
    body: PaymentsExternalAuthenticationRequest,
) -> Optional[Union[Any, PaymentsExternalAuthenticationResponse]]:
    """Payments - External 3DS Authentication

     External 3DS Authentication is performed and returns the AuthenticationResponse

    Args:
        payment_id (str):
        body (PaymentsExternalAuthenticationRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PaymentsExternalAuthenticationResponse]
    """

    return sync_detailed(
        payment_id=payment_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    payment_id: str,
    *,
    client: AuthenticatedClient,
    body: PaymentsExternalAuthenticationRequest,
) -> Response[Union[Any, PaymentsExternalAuthenticationResponse]]:
    """Payments - External 3DS Authentication

     External 3DS Authentication is performed and returns the AuthenticationResponse

    Args:
        payment_id (str):
        body (PaymentsExternalAuthenticationRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, PaymentsExternalAuthenticationResponse]]
    """

    kwargs = _get_kwargs(
        payment_id=payment_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    payment_id: str,
    *,
    client: AuthenticatedClient,
    body: PaymentsExternalAuthenticationRequest,
) -> Optional[Union[Any, PaymentsExternalAuthenticationResponse]]:
    """Payments - External 3DS Authentication

     External 3DS Authentication is performed and returns the AuthenticationResponse

    Args:
        payment_id (str):
        body (PaymentsExternalAuthenticationRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, PaymentsExternalAuthenticationResponse]
    """

    return (
        await asyncio_detailed(
            payment_id=payment_id,
            client=client,
            body=body,
        )
    ).parsed
