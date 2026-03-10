import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Auto Retries & Step Up 3DS", () => {
  // Restore the session if it exists
  beforeEach(() => {
    cy.session("login", () => {
      // Make sure we have credentials
      if (!globalState.get("email") || !globalState.get("password")) {
        throw new Error("Missing login credentials in global state");
      }

      cy.userLogin(globalState)
        .then(() => cy.terminate2Fa(globalState))
        .then(() => cy.userInfo(globalState))
        .then(() => {
          // Verify we have all necessary tokens and IDs
          const requiredKeys = [
            "userInfoToken",
            "merchantId",
            "organizationId",
            "profileId",
          ];
          requiredKeys.forEach((key) => {
            if (!globalState.get(key)) {
              throw new Error(`Missing required key after login: ${key}`);
            }
          });
        });
    });
  });

  context("Get merchant info", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("List MCA", () => {
      cy.ListMcaByMid(globalState);
    });

    it("API key create call", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("Customer create call", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Retrieve Merchant", () => {
      cy.merchantRetrieveCall(globalState);
    });
  });

  context("Auto Retries", () => {
    context("[Config: enable] Auto retries", () => {
      it("Enable auto retries", () => {
        const merchantId = globalState.get("merchantId");
        cy.setConfigs(
          globalState,
          `should_call_gsm_${merchantId}`,
          "true",
          "UPDATE"
        );
      });

      context("Max auto retries", () => {
        context("Adyen -> Stripe auto retries", () => {
          context("Enable routing configs", () => {
            before("seed global state", () => {
              cy.task("getGlobalState").then((state) => {
                globalState = new State(state);
              });
            });

            afterEach("flush global state", () => {
              cy.task("setGlobalState", globalState.data);
            });

            it("Add routing config", () => {
              const data =
                utils.getConnectorDetails("common")["priorityRouting"];
              const routing_data = [
                {
                  connector: "adyen",
                  merchant_connector_id: globalState.get("adyenMcaId"),
                },
                {
                  connector: "stripe",
                  merchant_connector_id: globalState.get("stripeMcaId"),
                },
                {
                  connector: "bluesnap",
                  merchant_connector_id: globalState.get("bluesnapMcaId"),
                },
              ];
              cy.addRoutingConfig(
                fixtures.routingConfigBody,
                data,
                "priority",
                routing_data,
                globalState
              );
            });

            it("Activate routing config", () => {
              const data =
                utils.getConnectorDetails("common")["priorityRouting"];

              cy.activateRoutingConfig(data, globalState);
            });
          });

          context("Max auto retries = 2", () => {
            const maxAutoRetries = 2;
            it("Update max auto retries", () => {
              const merchantId = globalState.get("merchantId");
              cy.setConfigs(
                globalState,
                `max_auto_retries_enabled_${merchantId}`,
                `${maxAutoRetries}`,
                "UPDATE"
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "PaymentIntent"
                  ];

                cy.createPaymentIntentTest(
                  fixtures.createPaymentBody,
                  data,
                  "no_three_ds",
                  "automatic",
                  globalState
                );
              });

              it("Payment confirm call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "BluesnapConfirm"
                  ];

                cy.confirmCallTest(
                  fixtures.confirmBody,
                  data,
                  true,
                  globalState
                );
              });

              it("Payment retrieve call", () => {
                cy.retrievePaymentCallTest(
                  globalState,
                  null,
                  true,
                  maxAutoRetries + 1
                );
              });
            });
          });

          context("Max auto retries = 1", () => {
            const maxAutoRetries = 1;
            it("Update max auto retries", () => {
              const merchantId = globalState.get("merchantId");
              cy.setConfigs(
                globalState,
                `max_auto_retries_enabled_${merchantId}`,
                `${maxAutoRetries}`,
                "UPDATE"
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "PaymentIntent"
                  ];

                cy.createPaymentIntentTest(
                  fixtures.createPaymentBody,
                  data,
                  "no_three_ds",
                  "automatic",
                  globalState
                );
              });

              it("Payment confirm call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "StripeConfirmSuccess"
                  ];

                cy.confirmCallTest(
                  fixtures.confirmBody,
                  data,
                  true,
                  globalState
                );
              });

              it("Payment retrieve call", () => {
                cy.retrievePaymentCallTest(
                  globalState,
                  null,
                  true,
                  maxAutoRetries + 1
                );
              });
            });
          });
          context("Max auto retries = 0", () => {
            const maxAutoRetries = 0;
            it("Update max auto retries", () => {
              const merchantId = globalState.get("merchantId");
              cy.setConfigs(
                globalState,
                `max_auto_retries_enabled_${merchantId}`,
                `${maxAutoRetries}`,
                "UPDATE"
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "PaymentIntent"
                  ];

                cy.createPaymentIntentTest(
                  fixtures.createPaymentBody,
                  data,
                  "no_three_ds",
                  "automatic",
                  globalState
                );
              });

              it("Payment confirm call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "AdyenConfirmFail"
                  ];

                cy.confirmCallTest(
                  fixtures.confirmBody,
                  data,
                  true,
                  globalState
                );
              });

              it("Payment retrieve call", () => {
                cy.retrievePaymentCallTest(
                  globalState,
                  null,
                  true,
                  maxAutoRetries + 1
                );
              });
            });
          });
        });

        context("Stripe -> Adyen auto retries", () => {
          context("Enable routing configs", () => {
            before("seed global state", () => {
              cy.task("getGlobalState").then((state) => {
                globalState = new State(state);
              });
            });

            afterEach("flush global state", () => {
              cy.task("setGlobalState", globalState.data);
            });

            it("Add routing config", () => {
              const data =
                utils.getConnectorDetails("common")["priorityRouting"];
              const routing_data = [
                {
                  connector: "stripe",
                  merchant_connector_id: globalState.get("stripeMcaId"),
                },
                {
                  connector: "adyen",
                  merchant_connector_id: globalState.get("adyenMcaId"),
                },
                {
                  connector: "bluesnap",
                  merchant_connector_id: globalState.get("bluesnapMcaId"),
                },
              ];
              cy.addRoutingConfig(
                fixtures.routingConfigBody,
                data,
                "priority",
                routing_data,
                globalState
              );
            });

            it("Activate routing config", () => {
              const data =
                utils.getConnectorDetails("common")["priorityRouting"];

              cy.activateRoutingConfig(data, globalState);
            });
          });

          context("Max auto retries = 2", () => {
            const maxAutoRetries = 2;
            it("Update max auto retries", () => {
              const merchantId = globalState.get("merchantId");
              cy.setConfigs(
                globalState,
                `max_auto_retries_enabled_${merchantId}`,
                `${maxAutoRetries}`,
                "UPDATE"
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "PaymentIntent"
                  ];

                cy.createPaymentIntentTest(
                  fixtures.createPaymentBody,
                  data,
                  "no_three_ds",
                  "automatic",
                  globalState
                );
              });

              it("Payment confirm call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "BluesnapConfirm"
                  ];

                cy.confirmCallTest(
                  fixtures.confirmBody,
                  data,
                  true,
                  globalState
                );
              });

              it("Payment retrieve call", () => {
                cy.retrievePaymentCallTest(
                  globalState,
                  null,
                  true,
                  maxAutoRetries + 1
                );
              });
            });
          });

          context("Max auto retries = 1", () => {
            const maxAutoRetries = 1;
            it("Update max auto retries", () => {
              const merchantId = globalState.get("merchantId");
              cy.setConfigs(
                globalState,
                `max_auto_retries_enabled_${merchantId}`,
                `${maxAutoRetries}`,
                "UPDATE"
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "PaymentIntent"
                  ];

                cy.createPaymentIntentTest(
                  fixtures.createPaymentBody,
                  data,
                  "no_three_ds",
                  "automatic",
                  globalState
                );
              });

              it("Payment confirm call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "AdyenConfirm"
                  ];

                cy.confirmCallTest(
                  fixtures.confirmBody,
                  data,
                  true,
                  globalState
                );
              });

              it("Payment retrieve call", () => {
                cy.retrievePaymentCallTest(
                  globalState,
                  null,
                  true,
                  maxAutoRetries + 1
                );
              });
            });
          });

          context("Max auto retries = 0", () => {
            const maxAutoRetries = 0;
            it("Update max auto retries", () => {
              const merchantId = globalState.get("merchantId");
              cy.setConfigs(
                globalState,
                `max_auto_retries_enabled_${merchantId}`,
                `${maxAutoRetries}`,
                "UPDATE"
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "PaymentIntent"
                  ];

                cy.createPaymentIntentTest(
                  fixtures.createPaymentBody,
                  data,
                  "no_three_ds",
                  "automatic",
                  globalState
                );
              });

              it("Payment confirm call", () => {
                const data =
                  utils.getConnectorDetails("autoretries")["card_pm"][
                    "StripeConfirmFail"
                  ];

                cy.confirmCallTest(
                  fixtures.confirmBody,
                  data,
                  true,
                  globalState
                );
              });

              it("Payment retrieve call", () => {
                cy.retrievePaymentCallTest(
                  globalState,
                  null,
                  true,
                  maxAutoRetries + 1
                );
              });
            });
          });
        });
      });

      context("Step up 3DS", () => {
        context("[Config: set] GSM", () => {
          it("[Config: enable] Step up GSM", () => {
            cy.updateGsmConfig(fixtures.gsmBody.gsm_update, globalState, true);
          });

          it("[Config: enable] Step up for Stripe", () => {
            const merchantId = globalState.get("merchantId");
            cy.setConfigs(
              globalState,
              `step_up_enabled_${merchantId}`,
              '["stripe"]',
              "UPDATE"
            );
          });
        });

        context("Make Payment", () => {
          const maxAutoRetries = 1;
          it("Update max auto retries", () => {
            const merchantId = globalState.get("merchantId");
            cy.setConfigs(
              globalState,
              `max_auto_retries_enabled_${merchantId}`,
              `${maxAutoRetries}`,
              "UPDATE"
            );
          });

          it("Payment create call", () => {
            const data =
              utils.getConnectorDetails("autoretries")["card_pm"][
                "PaymentIntent"
              ];

            cy.createPaymentIntentTest(
              fixtures.createPaymentBody,
              data,
              "no_three_ds",
              "automatic",
              globalState
            );
          });

          it("Payment confirm call", () => {
            const data =
              utils.getConnectorDetails("autoretries")["card_pm"][
                "StripeConfirm3DS"
              ];

            cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
          });

          it("Payment retrieve call", () => {
            cy.retrievePaymentCallTest(
              globalState,
              null,
              true,
              maxAutoRetries + 1
            );
          });
        });
      });
    });

    context("[Config: disable] Auto retries", () => {
      it("[Config: disable] Auto retries", () => {
        const merchantId = globalState.get("merchantId");
        cy.setConfigs(
          globalState,
          `should_call_gsm_${merchantId}`,
          "false",
          "UPDATE"
        );
      });

      it("[Config: disable] Step up GSM", () => {
        cy.updateGsmConfig(fixtures.gsmBody.gsm_update, globalState, false);
      });

      context("Make payment", () => {
        context("[Failed] Make payment", () => {
          it("Payment create call", () => {
            const data =
              utils.getConnectorDetails("autoretries")["card_pm"][
                "PaymentIntent"
              ];

            cy.createPaymentIntentTest(
              fixtures.createPaymentBody,
              data,
              "no_three_ds",
              "automatic",
              globalState
            );
          });

          it("Payment confirm call", () => {
            const data =
              utils.getConnectorDetails("autoretries")["card_pm"][
                "StripeConfirmFail"
              ];

            cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
          });

          it("Payment retrieve call", () => {
            cy.retrievePaymentCallTest(globalState, null, true);
          });
        });

        context("[Succeeded] Make payment", () => {
          it("Payment create call", () => {
            const data =
              utils.getConnectorDetails("autoretries")["card_pm"][
                "PaymentIntent"
              ];

            cy.createPaymentIntentTest(
              fixtures.createPaymentBody,
              data,
              "no_three_ds",
              "automatic",
              globalState
            );
          });

          it("Payment confirm call", () => {
            const data =
              utils.getConnectorDetails("autoretries")["card_pm"][
                "StripeConfirmSuccess"
              ];

            cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
          });

          it("Payment retrieve call", () => {
            cy.retrievePaymentCallTest(globalState, null, true);
          });
        });
      });
    });
  });
});
