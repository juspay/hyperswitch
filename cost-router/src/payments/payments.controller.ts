import { BadRequestException, Body, Controller, Get, Header, Headers, HttpCode, NotFoundException, Param, Post } from '@nestjs/common';
import { RoutingMetrics } from '../metrics/routing-metrics';
import { PaymentsService } from './payments.service';

type Json = Record<string, unknown>;

@Controller()
export class PaymentsController {
  constructor(
    private readonly payments: PaymentsService,
    private readonly metrics: RoutingMetrics,
  ) {}

  /** Prometheus scrape endpoint (text exposition format). */
  @Get('metrics')
  @Header('Content-Type', 'text/plain; version=0.0.4; charset=utf-8')
  async scrape(): Promise<string> {
    return this.metrics.render();
  }

  @Get('health')
  health(): { status: string } {
    return { status: 'ok' };
  }

  /**
   * Requires a stable payment id: `Idempotency-Key` header or body `payment_id`.
   * `x-cost-router-bin-class` / `x-cost-router-card-type` are this service's own optional hints
   * for fee matching; they are not forwarded and must never carry a card number.
   */
  @Post('payments')
  @HttpCode(200)
  async create(
    @Body() body: Json,
    @Headers('idempotency-key') idempotencyKey?: string,
    @Headers('x-cost-router-bin-class') binClass?: string,
    @Headers('x-cost-router-card-type') cardType?: string,
  ) {
    if (typeof body !== 'object' || body === null || Array.isArray(body)) {
      throw new BadRequestException('JSON object body required');
    }
    const bodyId = body['payment_id'];
    if (bodyId !== undefined && (typeof bodyId !== 'string' || bodyId.length === 0)) {
      throw new BadRequestException('payment_id must be a non-empty string');
    }
    if (idempotencyKey && bodyId && idempotencyKey !== bodyId) {
      throw new BadRequestException('Idempotency-Key and payment_id disagree');
    }
    const paymentId = idempotencyKey || (bodyId as string | undefined);
    if (!paymentId) throw new BadRequestException('Idempotency-Key header or payment_id is required');

    const { replayed, record } = await this.payments.create(paymentId, body, {
      ...(binClass && { binClass }),
      ...(cardType && { cardType }),
    });
    const { response, fingerprint: _fp, ...routing } = record;
    return { replayed, routing, payment: response?.body ?? null };
  }

  /** P1 bonus: the stored decision for a payment. */
  // Also served under /v1 because the assignment names GET /v1/payments/{id}/routing-trace.
  @Get(['payments/:id/routing-trace', 'v1/payments/:id/routing-trace'])
  trace(@Param('id') id: string) {
    const record = this.payments.getTrace(id);
    if (!record) throw new NotFoundException(`no routing decision stored for ${id}: unknown id, or expired / evicted from the bounded decision store`);
    const { response: _response, fingerprint: _fp, ...routing } = record;
    return routing;
  }
}
