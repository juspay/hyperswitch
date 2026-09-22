# Hyperswitch CommerceTools Extension

A commercetools HTTP API extension that integrates with Hyperswitch for payment processing.

## Overview

This extension enables commercetools merchants to process payments through Hyperswitch, connecting to various payment providers supported by the Hyperswitch platform.

## Features

- Payment creation and processing via Hyperswitch
- Support for authorization, capture, and refund operations
- Webhook handling for payment status updates
- Multi-tenancy support for multiple commercetools projects
- Proxy support for outgoing API requests
- Comprehensive error handling and logging

## Architecture

The extension consists of two main components:

1. **HTTP API Extension**: Processes payment creation/update requests from commercetools
2. **Webhook Handler**: Receives notifications from Hyperswitch and updates commercetools payments

## Installation

### Prerequisites

- Node.js 18+ or Python 3.9+
- A commercetools project with API credentials
- A Hyperswitch merchant account and API keys
- Publicly accessible endpoint for webhooks

### Deployment Options

1. **Cloud Functions**: Deploy to AWS Lambda, Google Cloud Functions, or Azure Functions
2. **Container**: Deploy as a Docker container to any container platform
3. **Server**: Traditional server deployment

## Configuration

### Environment Variables

Required environment variables:

```
HYPERSWITCH_API_KEY=your_hyperswitch_api_key
HYPERSWITCH_BASE_URL=https://sandbox.hyperswitch.io
COMMERCETOOLS_PROJECT_KEY=your_project_key
COMMERCETOOLS_CLIENT_ID=your_client_id
COMMERCETOOLS_CLIENT_SECRET=your_client_secret
COMMERCETOOLS_API_URL=https://api.europe-west1.gcp.commercetools.com
WEBHOOK_SECRET=your_webhook_secret
HTTP_PROXY=optional_proxy_url
HTTPS_PROXY=optional_proxy_url
```

### commercetools Extension Configuration

Configure the extension in your commercetools project:

1. Create an API Extension for payment interactions
2. Set the extension URL to your deployed service endpoint
3. Configure the extension to trigger on:
   - Payment creation
   - Payment update actions (capture, refund, cancel)

## Usage

Once configured, the extension will automatically:
- Create payments in Hyperswitch when payments are created in commercetools
- Update payment status in commercetools based on Hyperswitch webhook notifications
- Process capture, refund, and cancel operations

## Development

### Local Development

1. Clone the repository
2. Install dependencies: `npm install` or `pip install -r requirements.txt`
3. Set environment variables in `.env` file
4. Run the development server: `npm run dev` or `python app.py`
5. Use ngrok or similar tool to expose local server for webhook testing

### Testing

Run tests with: `npm test` or `pytest`

## Security

- All API keys and secrets are stored as environment variables
- Webhook signatures are verified using HMAC
- Sensitive data is never logged
- HTTPS is required for all endpoints

## Support

For issues and questions:
- Check the [Hyperswitch documentation](https://api-reference.hyperswitch.io)
- Refer to [commercetools API documentation](https://docs.commercetools.com)
- Open an issue in the GitHub repository

## License

MIT
