import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Platform Customer Flows", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    globalState.set("customerId", globalState.get("customerIdCm1Created"));
    cy.task("setGlobalState", globalState.data);
  });

  context("Shared Customer Across Connected Merchants", () => {
    it("create-shared-customer-using-platform-merchant", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("verify-connected-merchant-1-can-access-shared-customer", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      cy.customerRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("verify-connected-merchant-2-can-access-shared-customer", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKeyCm2"));

      cy.customerRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("CM1 Creates Customer - Accessible by CM2 and Platform", () => {
    it("cm1-creates-customer", () => {
      const savedApiKey = globalState.get("apiKey");
      const savedMerchantId = globalState.get("merchantId");
      globalState.set("apiKey", globalState.get("apiKeyCm1"));
      globalState.set("merchantId", globalState.get("connectedMerchantId1"));

      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

      cy.then(() => {
        globalState.set("customerIdCm1Created", globalState.get("customerId"));
        globalState.set("apiKey", savedApiKey);
        globalState.set("merchantId", savedMerchantId);
      });
    });

    it("verify-connected-merchant-2-can-access-cm1-customer", () => {
      const savedApiKey = globalState.get("apiKey");
      const savedCustomerId = globalState.get("customerId");
      globalState.set("apiKey", globalState.get("apiKeyCm2"));
      globalState.set("customerId", globalState.get("customerIdCm1Created"));

      cy.customerRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("customerId", savedCustomerId);
      });
    });

    it("verify-platform-merchant-can-access-cm1-customer", () => {
      const savedApiKey = globalState.get("apiKey");
      const savedCustomerId = globalState.get("customerId");
      globalState.set("customerId", globalState.get("customerIdCm1Created"));

      cy.customerRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("customerId", savedCustomerId);
      });
    });
  });

  context("Standard Merchant Cannot Access Shared Customer", () => {
    it("standard-merchant-cannot-retrieve-shared-customer", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.customerRetrieveCall(globalState, 404);

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context(
    "Standard Merchant Creates Customer - Only Accessible by Standard",
    () => {
      it("standard-merchant-creates-customer", () => {
        const savedApiKey = globalState.get("apiKey");
        const savedMerchantId = globalState.get("merchantId");
        globalState.set("apiKey", globalState.get("apiKeySm"));
        globalState.set("merchantId", globalState.get("standardMerchantId"));

        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

        cy.then(() => {
          globalState.set("customerIdSmCreated", globalState.get("customerId"));
          globalState.set("apiKey", savedApiKey);
          globalState.set("merchantId", savedMerchantId);
        });
      });

      it("verify-platform-merchant-cannot-access-standard-customer", () => {
        const savedApiKey = globalState.get("apiKey");
        const savedCustomerId = globalState.get("customerId");
        globalState.set("customerId", globalState.get("customerIdSmCreated"));

        cy.customerRetrieveCall(globalState, 404);

        cy.then(() => {
          globalState.set("apiKey", savedApiKey);
          globalState.set("customerId", savedCustomerId);
        });
      });

      it("verify-connected-merchant-1-cannot-access-standard-customer", () => {
        const savedApiKey = globalState.get("apiKey");
        const savedCustomerId = globalState.get("customerId");
        globalState.set("apiKey", globalState.get("apiKeyCm1"));
        globalState.set("customerId", globalState.get("customerIdSmCreated"));

        cy.customerRetrieveCall(globalState, 404);

        cy.then(() => {
          globalState.set("apiKey", savedApiKey);
          globalState.set("customerId", savedCustomerId);
        });
      });

      it("verify-connected-merchant-2-cannot-access-standard-customer", () => {
        const savedApiKey = globalState.get("apiKey");
        const savedCustomerId = globalState.get("customerId");
        globalState.set("apiKey", globalState.get("apiKeyCm2"));
        globalState.set("customerId", globalState.get("customerIdSmCreated"));

        cy.customerRetrieveCall(globalState, 404);

        cy.then(() => {
          globalState.set("apiKey", savedApiKey);
          globalState.set("customerId", savedCustomerId);
        });
      });

      it("verify-standard-merchant-can-access-its-own-customer", () => {
        const savedApiKey = globalState.get("apiKey");
        const savedCustomerId = globalState.get("customerId");
        globalState.set("apiKey", globalState.get("apiKeySm"));
        globalState.set("customerId", globalState.get("customerIdSmCreated"));

        cy.customerRetrieveCall(globalState);

        cy.then(() => {
          globalState.set("apiKey", savedApiKey);
          globalState.set("customerId", savedCustomerId);
        });
      });
    }
  );
});
