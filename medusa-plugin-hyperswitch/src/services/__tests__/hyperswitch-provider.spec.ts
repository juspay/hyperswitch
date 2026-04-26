import { HyperswitchPaymentProvider } from "../hyperswitch-payment-provider";

const mockConfig = {
  apiKey: "test_key",
  baseUrl: "https://api.hyperswitch.io",
  merchantId: "test_merchant"
};

describe("HyperswitchPaymentProvider", () => {
  let provider: HyperswitchPaymentProvider;

  beforeEach(() => {
    provider = new HyperswitchPaymentProvider({}, mockConfig);
  });

  it("should initialize correctly with config", () => {
    expect(provider).toBeDefined();
  });

  it("should handle authorizePayment with success", async () => {
    const cart = { id: "cart_123", total: 1000 } as any;
    const result = await provider.authorizePayment(cart, {}, {});
    expect(result).toBeDefined();
    expect(result.status).toBe("authorized");
  });

  it("should handle refundPayment correctly", async () => {
    const paymentData = { id: "pay_123" } as any;
    const result = await provider.refundPayment(paymentData, 500);
    expect(result).toBeDefined();
    expect(result.id).toBe("pay_123");
  });
});
