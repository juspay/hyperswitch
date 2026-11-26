/**
 * Hyperswitch CommerceTools Extension
 * HTTP API Extension handler for commercetools payment processing
 */

const crypto = require('crypto');
const axios = require('axios');
const https = require('https');

// Configure HTTP client with proxy support
const createHttpClient = (baseURL) => {
    const proxy = process.env.HTTP_PROXY || process.env.HTTPS_PROXY;
    const config = {
        baseURL,
        timeout: 30000,
        headers: {
            'Content-Type': 'application/json',
            'api-key': process.env.HYPERSWITCH_API_KEY
        }
    };

    if (proxy) {
        const HttpsProxyAgent = require('https-proxy-agent');
        config.httpsAgent = new HttpsProxyAgent(proxy);
        config.proxy = false;
    } else {
        config.httpsAgent = new https.Agent({ keepAlive: true });
    }

    return axios.create(config);
};

class HyperswitchClient {
    constructor() {
        this.baseURL = process.env.HYPERSWITCH_BASE_URL || 'https://sandbox.hyperswitch.io';
        this.client = createHttpClient(this.baseURL);
    }

    async createPayment(paymentData) {
        try {
            const response = await this.client.post('/payments', paymentData);
            return response.data;
        } catch (error) {
            console.error('Hyperswitch createPayment error:', error.response?.data || error.message);
            throw this.normalizeError(error);
        }
    }

    async capturePayment(paymentId, amount) {
        try {
            const response = await this.client.post(`/payments/${paymentId}/capture`, {
                amount: amount
            });
            return response.data;
        } catch (error) {
            console.error('Hyperswitch capturePayment error:', error.response?.data || error.message);
            throw this.normalizeError(error);
        }
    }

    async refundPayment(paymentId, amount) {
        try {
            const response = await this.client.post(`/payments/${paymentId}/refund`, {
                amount: amount
            });
            return response.data;
        } catch (error) {
            console.error('Hyperswitch refundPayment error:', error.response?.data || error.message);
            throw this.normalizeError(error);
        }
    }

    async cancelPayment(paymentId) {
        try {
            const response = await this.client.post(`/payments/${paymentId}/cancel`);
            return response.data;
        } catch (error) {
            console.error('Hyperswitch cancelPayment error:', error.response?.data || error.message);
            throw this.normalizeError(error);
        }
    }

    async getPayment(paymentId) {
        try {
            const response = await this.client.get(`/payments/${paymentId}`);
            return response.data;
        } catch (error) {
            console.error('Hyperswitch getPayment error:', error.response?.data || error.message);
            throw this.normalizeError(error);
        }
    }

    normalizeError(error) {
        if (error.response) {
            return {
                status: error.response.status,
                code: error.response.data?.error?.code || 'api_error',
                message: error.response.data?.error?.message || error.message,
                details: error.response.data
            };
        }
        return {
            status: 500,
            code: 'internal_error',
            message: error.message
        };
    }
}

class CommerceToolsClient {
    constructor(projectKey) {
        this.projectKey = projectKey;
        this.baseURL = process.env.COMMERCETOOLS_API_URL || 'https://api.europe-west1.gcp.commercetools.com';
        this.client = axios.create({
            baseURL: `${this.baseURL}/${this.projectKey}`,
            auth: {
                username: process.env.COMMERCETOOLS_CLIENT_ID,
                password: process.env.COMMERCETOOLS_CLIENT_SECRET
            },
            headers: {
                'Content-Type': 'application/json'
            }
        });
    }

    async updatePayment(paymentId, version, actions) {
        try {
            const response = await this.client.post(`/payments/${paymentId}`, {
                version,
                actions
            });
            return response.data;
        } catch (error) {
            console.error('CommerceTools updatePayment error:', error.response?.data || error.message);
            throw error;
        }
    }
}

class PaymentMapper {
    static toHyperswitch(ctPayment) {
        const amount = ctPayment.amountPlanned?.centAmount || 0;
        const currency = ctPayment.amountPlanned?.currencyCode || 'USD';
        
        return {
            amount: Math.abs(amount), // Ensure positive amount
            currency: currency,
            customer_id: ctPayment.customer?.id || `ct-${ctPayment.id}`,
            merchant_order_id: ctPayment.id,
            metadata: {
                commercetools_payment_id: ctPayment.id,
                commercetools_version: ctPayment.version
            },
            payment_method: ctPayment.paymentMethodInfo?.method || 'card',
            payment_method_data: this.mapPaymentMethodData(ctPayment),
            return_url: this.findReturnUrl(ctPayment),
            description: `Payment for order ${ctPayment.interfaceId || ctPayment.id}`
        };
    }

    static mapPaymentMethodData(ctPayment) {
        const method = ctPayment.paymentMethodInfo?.method;
        if (method === 'card' && ctPayment.paymentMethodInfo?.paymentInterface === 'hyperswitch') {
            // Extract card data from custom fields or transactions
            const card = ctPayment.custom?.fields?.card || {};
            return {
                card: {
                    card_number: card.number,
                    card_exp_month: card.expMonth,
                    card_exp_year: card.expYear,
                    card_holder_name: card.holderName,
                    card_cvc: card.cvc
                }
            };
        }
        return {};
    }

    static findReturnUrl(ctPayment) {
        // Look for return URL in custom fields or interface interactions
        return ctPayment.custom?.fields?.returnUrl || 
               ctPayment.interfaceInteractions?.find(i => i.type === 'returnUrl')?.fields?.url ||
               'https://example.com/return';
    }

    static toCommerceTools(hyperswitchPayment, ctPayment) {
        const actions = [];
        const status = hyperswitchPayment.status;
        
        // Map Hyperswitch status to commercetools transaction type
        switch (status) {
            case 'requires_customer_action':
            case 'requires_confirmation':
                actions.push({
                    action: 'addTransaction',
                    transaction: {
                        type: 'Authorization',
                        amount: {
                            centAmount: hyperswitchPayment.amount,
                            currencyCode: hyperswitchPayment.currency
                        },
                        state: 'Pending',
                        interactionId: hyperswitchPayment.payment_id
                    }
                });
                break;
            case 'succeeded':
                actions.push({
                    action: 'addTransaction',
                    transaction: {
                        type: 'Charge',
                        amount: {
                            centAmount: hyperswitchPayment.amount,
                            currencyCode: hyperswitchPayment.currency
                        },
                        state: 'Success',
                        interactionId: hyperswitchPayment.payment_id,
                        timestamp: new Date().toISOString()
                    }
                });
                break;
            case 'failed':
                actions.push({
                    action: 'addTransaction',
                    transaction: {
                        type: 'Authorization',
                        amount: {
                            centAmount: hyperswitchPayment.amount,
                            currencyCode: hyperswitchPayment.currency
                        },
                        state: 'Failure',
                        interactionId: hyperswitchPayment.payment_id
                    }
                });
                break;
        }

        // Update payment method info if available
        if (hyperswitchPayment.payment_method) {
            actions.push({
                action: 'setMethodInfoMethod',
                method: hyperswitchPayment.payment_method
            });
        }

        // Store Hyperswitch payment ID in custom fields
        actions.push({
            action: 'setCustomField',
            name: 'hyperswitchPaymentId',
            value: hyperswitchPayment.payment_id
        });

        return actions;
    }
}

class ExtensionHandler {
    constructor() {
        this.hyperswitchClient = new HyperswitchClient();
        this.projectCache = new Map();
    }

    async handleRequest(request) {
        const { action, resource } = request;
        
        try {
            switch (action) {
                case 'Create':
                    return await this.handlePaymentCreate(request);
                case 'Update':
                    return await this.handlePaymentUpdate(request);
                default:
                    return this.createResponse(400, {
                        errors: [{ code: 'InvalidAction', message: `Unsupported action: ${action}` }]
                    });
            }
        } catch (error) {
            console.error('Extension handler error:', error);
            return this.createResponse(error.status || 500, {
                errors: [{
                    code: error.code || 'InternalError',
                    message: error.message || 'Internal server error'
                }]
            });
        }
    }

    async handlePaymentCreate(request) {
        const ctPayment = request.resource.obj;
        const projectKey = this.extractProjectKey(request);
        
        // Map commercetools payment to Hyperswitch format
        const hyperswitchPaymentData = PaymentMapper.toHyperswitch(ctPayment);
        
        // Create payment in Hyperswitch
        const hyperswitchPayment = await this.hyperswitchClient.createPayment(hyperswitchPaymentData);
        
        // Map response to commercetools actions
        const actions = PaymentMapper.toCommerceTools(hyperswitchPayment, ctPayment);
        
        // Store project key for this payment
        this.projectCache.set(ctPayment.id, projectKey);
        
        return this.createResponse(200, { actions });
    }

    async handlePaymentUpdate(request) {
        const ctPayment = request.resource.obj;
        const updateActions = request.resource.updateActions || [];
        
        // Find relevant update actions
        for (const updateAction of updateActions) {
            switch (updateAction.action) {
                case 'addTransaction':
                    const transaction = updateAction.transaction;
                    if (transaction.type === 'Charge' && transaction.state === 'Initial') {
                        // Handle capture
                        const hyperswitchPaymentId = ctPayment.custom?.fields?.hyperswitchPaymentId;
                        if (hyperswitchPaymentId) {
                            await this.hyperswitchClient.capturePayment(
                                hyperswitchPaymentId,
                                transaction.amount.centAmount
                            );
                        }
                    } else if (transaction.type === 'Refund' && transaction.state === 'Initial') {
                        // Handle refund
                        const hyperswitchPaymentId = ctPayment.custom?.fields?.hyperswitchPaymentId;
                        if (hyperswitchPaymentId) {
                            await this.hyperswitchClient.refundPayment(
                                hyperswitchPaymentId,
                                transaction.amount.centAmount
                            );
                        }
                    }
                    break;
                case 'cancelPayment':
                    const hyperswitchPaymentId = ctPayment.custom?.fields?.hyperswitchPaymentId;
                    if (hyperswitchPaymentId) {
                        await this.hyperswitchClient.cancelPayment(hyperswitchPaymentId);
                    }
                    break;
            }
        }
        
        return this.createResponse(200, { actions: [] });
    }

    extractProjectKey(request) {
        // Extract project key from request URL or headers
        const url = request.requestContext?.path || '';
        const match = url.match(/\/in-project\/([^\/]+)/);
        return match ? match[1] : process.env.COMMERCETOOLS_PROJECT_KEY;
    }

    createResponse(statusCode, body) {
        return {
            statusCode,
            body: JSON.stringify(body),
            headers: {
                'Content-Type': 'application/json'
            }
        };
    }
}

class WebhookHandler {
    constructor() {
        this.webhookSecret = process.env.WEBHOOK_SECRET;
    }

    verifySignature(payload, signature) {
        if (!this.webhookSecret) return true; // Skip verification if no secret configured
        
        const hmac = crypto.createHmac('sha256', this.webhookSecret);
        const digest = hmac.update(payload).digest('hex');
        return crypto.timingSafeEqual(Buffer.from(digest), Buffer.from(signature));
    }

    async handleWebhook(request) {
        const signature = request.headers['x-hyperswitch-signature'];
        const payload = typeof request.body === 'string' ? request.body : JSON.stringify(request.body);
        
        if (!this.verifySignature(payload, signature)) {
            return { statusCode: 401, body: 'Invalid signature' };
        }

        const event = typeof request.body === 'object' ? request.body : JSON.parse(payload);
        
        try {
            await this.processEvent(event);
            return { statusCode: 200, body: 'Webhook processed successfully' };
        } catch (error) {
            console.error('Webhook processing error:', error);
            return { statusCode: 500, body: 'Internal server error' };
        }
    }

    async processEvent(event) {
        const { type, data } = event;
        const paymentId = data.object?.payment_id || data.object?.id;
        
        if (!paymentId) {
            throw new Error('No payment ID in webhook event');
        }

        // In a real implementation, you would:
        // 1. Look up the commercetools payment ID from your database using hyperswitchPaymentId
        // 2. Get the project key for this payment
        // 3. Update the payment in commercetools
        
        console.log(`Processing webhook event: ${type} for payment ${paymentId}`);
        
        // This is a simplified implementation
        // In production, you would implement proper state management and retry logic
    }
}

// Main handler for AWS Lambda/Google Cloud Functions
exports.handler = async (event, context) => {
    const extensionHandler = new ExtensionHandler();
    const webhookHandler = new WebhookHandler();
    
    // Determine if this is a webhook or extension request
    const path = event.path || '';
    
    if (path.includes('/webhook')) {
        return await webhookHandler.handleWebhook({
            headers: event.headers,
            body: event.body
        });
    } else {
        const request = JSON.parse(event.body || '{}');
        return await extensionHandler.handleRequest(request);
    }
};

// For local development
if (require.main === module) {
    const express = require('express');
    const app = express();
    const port = process.env.PORT || 3000;

    app.use(express.json());

    const extensionHandler = new ExtensionHandler();
    const webhookHandler = new WebhookHandler();

    // Extension endpoint
    app.post('/extension', async (req, res) => {
        try {
            const result = await extensionHandler.handleRequest(req.body);
            res.status(result.statusCode).json(JSON.parse(result.body));
        } catch (error) {
            console.error('Extension endpoint error:', error);
            res.status(500).json({ errors: [{ code: 'InternalError', message: 'Internal server error' }] });
        }
    });

    // Webhook endpoint
    app.post('/webhook', async (req, res) => {
        try {
            const result = await webhookHandler.handleWebhook({
                headers: req.headers,
                body: req.body
            });
            res.status(result.statusCode).send(result.body);
        } catch (error) {
            console.error('Webhook endpoint error:', error);
            res.status(500).send('Internal server error');
        }
    });

    // Health check
    app.get('/health', (req, res) => {
        res.status(200).json({ status: 'ok', timestamp: new Date().toISOString() });
    });

    app.listen(port, () => {
        console.log(`Hyperswitch CommerceTools extension listening on port ${port}`);
    });
}
