# Configuration Guide

This guide explains how to configure and deploy the Hyperswitch CommerceTools extension.

## Environment Variables

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `HYPERSWITCH_API_KEY` | Your Hyperswitch API key | `sk_...` |
| `HYPERSWITCH_BASE_URL` | Hyperswitch API base URL | `https://sandbox.hyperswitch.io` |
| `COMMERCETOOLS_PROJECT_KEY` | Your commercetools project key | `my-project` |
| `COMMERCETOOLS_CLIENT_ID` | commercetools client ID | `client-id-here` |
| `COMMERCETOOLS_CLIENT_SECRET` | commercetools client secret | `client-secret-here` |
| `COMMERCETOOLS_API_URL` | commercetools API URL | `https://api.europe-west1.gcp.commercetools.com` |

### Optional Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `WEBHOOK_SECRET` | Secret for verifying webhook signatures | (none) |
| `HTTP_PROXY` | HTTP proxy URL | (none) |
| `HTTPS_PROXY` | HTTPS proxy URL | (none) |
| `PORT` | Server port | `3000` |
| `NODE_ENV` | Node environment | `production` |

## commercetools Configuration

### 1. Create API Extension

1. Go to your commercetools Merchant Center
2. Navigate to **Settings** → **Extensions**
3. Click **Create new extension**
4. Configure as follows:

**General Settings:**
- Name: `Hyperswitch Payment Extension`
- Description: `Process payments through Hyperswitch`

**Destination:**
- Type: `HTTP`
- URL: `https://your-deployed-service.com/extension`
- Authentication: `Authorization Header` (if required)

**Triggers:**
- Resource Type: `Payment`
- Actions: `Create`, `Update`

### 2. Configure Payment Method

1. In Merchant Center, go to **Settings** → **Payment Methods**
2. Create a new payment method:
   - Name: `Hyperswitch`
   - Method: `Credit Card` (or appropriate method)
   - Interface: `hyperswitch`

### 3. Set Custom Fields (Optional)

You can add custom fields to the Payment resource to store additional data:

```json
{
  "name": "hyperswitchPaymentId",
  "type": "String"
}
```

## Hyperswitch Configuration

### 1. Get API Credentials

1. Sign up for a Hyperswitch account at [hyperswitch.io](https://hyperswitch.io)
2. Obtain your API key from the dashboard
3. Configure your payment methods in Hyperswitch

### 2. Configure Webhooks

1. In Hyperswitch dashboard, go to **Webhooks**
2. Add a new webhook endpoint:
   - URL: `https://your-deployed-service.com/webhook`
   - Events: Select all payment events
   - Secret: Set a webhook secret (must match `WEBHOOK_SECRET`)

## Deployment

### Option 1: Docker Deployment

```bash
# Build the image
docker build -t hyperswitch-extension .

# Run the container
docker run -p 3000:3000 \
  -e HYPERSWITCH_API_KEY=your_key \
  -e COMMERCETOOLS_PROJECT_KEY=your_project \
  -e COMMERCETOOLS_CLIENT_ID=your_client_id \
  -e COMMERCETOOLS_CLIENT_SECRET=your_client_secret \
  hyperswitch-extension
```

### Option 2: Docker Compose

```bash
# Create .env file with your variables
cp .env.example .env
# Edit .env with your values

# Start the service
docker-compose up -d
```

### Option 3: Serverless Deployment (AWS Lambda)

1. Package the extension:
```bash
npm install
zip -r extension.zip package.json src/ node_modules/
```

2. Upload to AWS Lambda
3. Configure API Gateway to route requests to Lambda
4. Set environment variables in Lambda configuration

### Option 4: Traditional Server

```bash
# Install dependencies
npm install

# Set environment variables
export HYPERSWITCH_API_KEY=your_key
export COMMERCETOOLS_PROJECT_KEY=your_project
# ... other variables

# Start the server
npm start
```

## Testing

### Local Testing with ngrok

1. Start the extension locally:
```bash
npm run dev
```

2. Expose local server:
```bash
ngrok http 3000
```

3. Use the ngrok URL in your commercetools extension configuration

### Test Webhooks

Use the Hyperswitch dashboard to send test webhook events, or use curl:

```bash
curl -X POST https://your-service.com/webhook \
  -H "Content-Type: application/json" \
  -H "x-hyperswitch-signature: generated_signature" \
  -d '{
    "type": "payment.succeeded",
    "data": {
      "object": {
        "payment_id": "pay_123",
        "status": "succeeded"
      }
    }
  }'
```

## Monitoring

### Health Check

The extension provides a health endpoint:
```
GET /health
```

### Logging

Logs are written to stdout/stderr. In production, configure log aggregation based on your deployment platform.

## Troubleshooting

### Common Issues

1. **Extension not triggering**
   - Check commercetools extension configuration
   - Verify the extension URL is accessible
   - Check commercetools project permissions

2. **Payment creation fails**
   - Verify Hyperswitch API key is valid
   - Check payment method configuration in Hyperswitch
   - Review request/response logs

3. **Webhooks not received**
   - Verify webhook URL is publicly accessible
   - Check webhook secret configuration
   - Review firewall/proxy settings

### Getting Help

- Check the [Hyperswitch API documentation](https://api-reference.hyperswitch.io)
- Refer to [commercetools API documentation](https://docs.commercetools.com)
- Open an issue in the GitHub repository
