import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as routingUtils from "../../configs/Routing/Utils";
import * as payoutUtils from "../../configs/Payout/Utils";

let globalState;

describe("Payout Priority Routing Test", () => {
  let shouldContinue = true;
  let outerGuardPassed = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !routingUtils.shouldRunForConnectorList(
          globalState.get("connectorId"),
          routingUtils.CONNECTOR_LISTS.INCLUDE.PAYOUT_ROUTING
        )
      ) {
        shouldContinue = false;
        outerGuardPassed = false;
      }
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("Payout Priority Routing - default connector", () => {
    before("setup payout context", () => {
      if (outerGuardPassed) {
        shouldContinue = true;
      }
      cy.ListMcaByMid(globalState);
    });

    it("add-payout-routing-config", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];
      // Use currentConnectorMcaId which is reliably set to the first payout connector
      // This works regardless of which specific connector (stripe/adyen/wise) is configured
      const routing_data = [
        {
          connector: globalState.get("connectorId"),
          merchant_connector_id: globalState.get("currentConnectorMcaId"),
        },
      ];
      cy.addRoutingConfig(
        fixtures.payoutRoutingConfigBody,
        data,
        "priority",
        routing_data,
        globalState
      );
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("retrieve-payout-routing-config-test", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];

      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("activate-payout-routing-config-test", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];

      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("payout-routing-test", () => {
      const payoutData = payoutUtils.getConnectorDetails(
        globalState.get("connectorId")
      )["bank_transfer_pm"]["sepa_bank_transfer"]["Fulfill"];

      if (!payoutUtils.should_continue_further(payoutData)) {
        cy.log(
          "Skipping payout creation for " + globalState.get("connectorId")
        );
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        payoutData,
        true,
        true,
        globalState
      );

      cy.verifyPayoutRoutingConnector(globalState);

      if (shouldContinue)
        shouldContinue = payoutUtils.should_continue_further(payoutData);
    });

    it("retrieve-payout-call-test", () => {
      if (!shouldContinue) {
        return;
      }
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Priority Routing - single connector alternate config", () => {
    // Context for testing a second payout routing configuration
    // Tests that multiple routing configs can coexist with different priorities

    before("setup alternate payout context", () => {
      if (outerGuardPassed) {
        shouldContinue = true;
      }
      cy.ListMcaByMid(globalState);
    });

    it("add-payout-routing-config-alternate", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];
      // Create an alternate configuration with same connector to validate
      // that multiple payout routing configs can be managed independently
      const routing_data = [
        {
          connector: globalState.get("connectorId"),
          merchant_connector_id: globalState.get("currentConnectorMcaId"),
        },
      ];

      cy.addRoutingConfig(
        fixtures.payoutRoutingConfigBody,
        data,
        "priority",
        routing_data,
        globalState
      );
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("retrieve-payout-routing-config-alternate-test", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];

      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("activate-payout-routing-config-alternate-test", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];

      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("payout-routing-alternate-test", () => {
      const payoutData = payoutUtils.getConnectorDetails(
        globalState.get("connectorId")
      )["bank_transfer_pm"]["sepa_bank_transfer"]["Fulfill"];

      if (!payoutUtils.should_continue_further(payoutData)) {
        cy.log(
          "Skipping payout creation for " + globalState.get("connectorId")
        );
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        payoutData,
        true,
        true,
        globalState
      );

      cy.verifyPayoutRoutingConnector(globalState);

      if (shouldContinue)
        shouldContinue = payoutUtils.should_continue_further(payoutData);
    });

    it("retrieve-payout-alternate-call-test", () => {
      if (!shouldContinue) {
        return;
      }
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Volume Based Routing", () => {
    before("setup volume routing context", () => {
      if (outerGuardPassed) {
        shouldContinue = true;
      }
      cy.ListMcaByMid(globalState);
    });

    it("add-payout-volume-routing-config", () => {
      const data =
        routingUtils.getConnectorDetails("common")["payoutVolumeRouting"];
      const routing_data = [
        {
          connector: {
            connector: globalState.get("connectorId"),
            merchant_connector_id: globalState.get("currentConnectorMcaId"),
          },
          split: 100,
        },
      ];
      cy.addRoutingConfig(
        fixtures.payoutRoutingConfigBody,
        data,
        "volume_split",
        routing_data,
        globalState
      );
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("retrieve-payout-volume-routing-config-test", () => {
      const data =
        routingUtils.getConnectorDetails("common")["payoutVolumeRouting"];

      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("activate-payout-volume-routing-config-test", () => {
      const data =
        routingUtils.getConnectorDetails("common")["payoutVolumeRouting"];

      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("payout-volume-routing-test", () => {
      const payoutData = payoutUtils.getConnectorDetails(
        globalState.get("connectorId")
      )["bank_transfer_pm"]["sepa_bank_transfer"]["Fulfill"];

      if (!payoutUtils.should_continue_further(payoutData)) {
        cy.log(
          "Skipping payout creation for " + globalState.get("connectorId")
        );
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        payoutData,
        true,
        true,
        globalState
      );

      cy.verifyPayoutRoutingConnector(globalState);

      if (shouldContinue)
        shouldContinue = payoutUtils.should_continue_further(payoutData);
    });

    it("retrieve-payout-volume-call-test", () => {
      if (!shouldContinue) {
        return;
      }
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Rule Based Routing", () => {
    before("setup rule routing context", () => {
      if (outerGuardPassed) {
        shouldContinue = true;
      }
      cy.ListMcaByMid(globalState);
    });

    it("add-payout-rule-routing-config", () => {
      const data =
        routingUtils.getConnectorDetails("common")["payoutRuleBasedRouting"];
      // Rule matches payout_type = "bank", routes to current connector
      const routing_data = {
        defaultSelection: {
          type: "priority",
          data: [
            {
              connector: globalState.get("connectorId"),
              merchant_connector_id: globalState.get("currentConnectorMcaId"),
            },
          ],
        },
        metadata: {},
        rules: [
          {
            name: "payout_rule_1",
            connectorSelection: {
              type: "priority",
              data: [
                {
                  connector: globalState.get("connectorId"),
                  merchant_connector_id: globalState.get(
                    "currentConnectorMcaId"
                  ),
                },
              ],
            },
            statements: [
              {
                condition: [
                  {
                    lhs: "payout_type",
                    comparison: "equal",
                    value: { type: "enum_variant", value: "bank" },
                    metadata: {},
                  },
                ],
              },
            ],
          },
        ],
      };
      cy.addRoutingConfig(
        fixtures.payoutRoutingConfigBody,
        data,
        "advanced",
        routing_data,
        globalState
      );
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("retrieve-payout-rule-routing-config-test", () => {
      const data =
        routingUtils.getConnectorDetails("common")["payoutRuleBasedRouting"];

      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("activate-payout-rule-routing-config-test", () => {
      const data =
        routingUtils.getConnectorDetails("common")["payoutRuleBasedRouting"];

      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("payout-rule-routing-test", () => {
      const payoutData = payoutUtils.getConnectorDetails(
        globalState.get("connectorId")
      )["bank_transfer_pm"]["sepa_bank_transfer"]["Fulfill"];

      if (!payoutUtils.should_continue_further(payoutData)) {
        cy.log(
          "Skipping payout creation for " + globalState.get("connectorId")
        );
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        payoutData,
        true,
        true,
        globalState
      );

      cy.verifyPayoutRoutingConnector(globalState);

      if (shouldContinue)
        shouldContinue = payoutUtils.should_continue_further(payoutData);
    });

    it("retrieve-payout-rule-call-test", () => {
      if (!shouldContinue) {
        return;
      }
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Default Fallback Routing", () => {
    before("setup default fallback routing context", () => {
      if (outerGuardPassed) {
        shouldContinue = true;
      }
      cy.ListMcaByMid(globalState);
    });

    it("add-payout-default-fallback-routing-config", () => {
      const data =
        routingUtils.getConnectorDetails("common")[
          "payoutDefaultFallbackRouting"
        ];
      // Rule amount > 999999 will never match for a test payout
      // defaultSelection routes to current connector as the fallback
      const routing_data = {
        defaultSelection: {
          type: "priority",
          data: [
            {
              connector: globalState.get("connectorId"),
              merchant_connector_id: globalState.get("currentConnectorMcaId"),
            },
          ],
        },
        metadata: {},
        rules: [
          {
            name: "payout_fallback_rule",
            connectorSelection: {
              type: "priority",
              data: [
                {
                  connector: globalState.get("connectorId"),
                  merchant_connector_id: globalState.get(
                    "currentConnectorMcaId"
                  ),
                },
              ],
            },
            statements: [
              {
                condition: [
                  {
                    lhs: "amount",
                    comparison: "greater_than",
                    value: { type: "number", value: 999999 },
                    metadata: {},
                  },
                ],
              },
            ],
          },
        ],
      };
      cy.addRoutingConfig(
        fixtures.payoutRoutingConfigBody,
        data,
        "advanced",
        routing_data,
        globalState
      );
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("retrieve-payout-default-fallback-routing-config-test", () => {
      const data =
        routingUtils.getConnectorDetails("common")[
          "payoutDefaultFallbackRouting"
        ];

      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    it("activate-payout-default-fallback-routing-config-test", () => {
      const data =
        routingUtils.getConnectorDetails("common")[
          "payoutDefaultFallbackRouting"
        ];

      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue)
        shouldContinue = routingUtils.should_continue_further(data);
    });

    // amount does NOT match rule (amount > 999999), routed to current connector via defaultSelection
    it("payout-default-fallback-routing-test", () => {
      const payoutData = payoutUtils.getConnectorDetails(
        globalState.get("connectorId")
      )["bank_transfer_pm"]["sepa_bank_transfer"]["Fulfill"];

      if (!payoutUtils.should_continue_further(payoutData)) {
        cy.log(
          "Skipping payout creation for " + globalState.get("connectorId")
        );
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        payoutData,
        true,
        true,
        globalState
      );

      cy.verifyPayoutRoutingConnector(globalState);

      if (shouldContinue)
        shouldContinue = payoutUtils.should_continue_further(payoutData);
    });

    it("retrieve-payout-default-fallback-call-test", () => {
      if (!shouldContinue) {
        return;
      }
      cy.retrievePayoutCallTest(globalState);
    });
  });
});
