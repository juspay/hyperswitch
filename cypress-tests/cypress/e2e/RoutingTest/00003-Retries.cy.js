import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import * as utils from "../RoutingUtils/Utils";

let globalState;

describe("Auto Retries & Step Up 3DS", () => {
  context("Login", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("User login", () => {
      cy.userLogin(globalState);
      cy.terminate2Fa(globalState);
      cy.userInfo(globalState);
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
        cy.updateConfig("autoRetry", fixtures.configs.gsm, globalState, "true");
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
              let data = utils.getConnectorDetails("common")["priorityRouting"];
              let routing_data = [
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
              let data = utils.getConnectorDetails("common")["priorityRouting"];

              cy.activateRoutingConfig(data, globalState);
            });
          });

          context("Max auto retries = 2", () => {
            const max_auto_retries = 2;
            it("Update max auto retries", () => {
              cy.updateConfig(
                "maxRetries",
                fixtures.configs.max_auto_retries,
                globalState,
                `${max_auto_retries}`
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                let data =
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
                let data =
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
                  max_auto_retries + 1
                );
              });
            });
          });

          context("Max auto retries = 1", () => {
            const max_auto_retries = 1;
            it("Update max auto retries", () => {
              cy.updateConfig(
                "maxRetries",
                fixtures.configs.max_auto_retries,
                globalState,
                `${max_auto_retries}`
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                let data =
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
                let data =
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
                  max_auto_retries + 1
                );
              });
            });
          });
          context("Max auto retries = 0", () => {
            const max_auto_retries = 0;
            it("Update max auto retries", () => {
              cy.updateConfig(
                "maxRetries",
                fixtures.configs.max_auto_retries,
                globalState,
                `${max_auto_retries}`
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                let data =
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
                let data =
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
                  max_auto_retries + 1
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
              let data = utils.getConnectorDetails("common")["priorityRouting"];
              let routing_data = [
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
              let data = utils.getConnectorDetails("common")["priorityRouting"];

              cy.activateRoutingConfig(data, globalState);
            });
          });

          context("Max auto retries = 2", () => {
            const max_auto_retries = 2;
            it("Update max auto retries", () => {
              cy.updateConfig(
                "maxRetries",
                fixtures.configs.max_auto_retries,
                globalState,
                `${max_auto_retries}`
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                let data =
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
                let data =
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
                  max_auto_retries + 1
                );
              });
            });
          });

          context("Max auto retries = 1", () => {
            const max_auto_retries = 1;
            it("Update max auto retries", () => {
              cy.updateConfig(
                "maxRetries",
                fixtures.configs.max_auto_retries,
                globalState,
                `${max_auto_retries}`
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                let data =
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
                let data =
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
                  max_auto_retries + 1
                );
              });
            });
          });

          context("Max auto retries = 0", () => {
            const max_auto_retries = 0;
            it("Update max auto retries", () => {
              cy.updateConfig(
                "maxRetries",
                fixtures.configs.max_auto_retries,
                globalState,
                `${max_auto_retries}`
              );
            });

            context("Make payment", () => {
              it("Payment create call", () => {
                let data =
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
                let data =
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
                  max_auto_retries + 1
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
            cy.updateConfig(
              "stepUp",
              fixtures.configs.step_up,
              globalState,
              '["stripe"]'
            );
          });
        });

        context("Make Payment", () => {
          const max_auto_retries = 1;
          it("Update max auto retries", () => {
            cy.updateConfig(
              "maxRetries",
              fixtures.configs.max_auto_retries,
              globalState,
              `${max_auto_retries}`
            );
          });

          it("Payment create call", () => {
            let data =
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
            let data =
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
              max_auto_retries + 1
            );
          });
        });
      });
    });

    context("[Config: disable] Auto retries", () => {
      it("[Config: disable] Auto retries", () => {
        cy.updateConfig(
          "autoRetry",
          fixtures.configs.gsm,
          globalState,
          "false"
        );
      });

      it("[Config: disable] Step up GSM", () => {
        cy.updateGsmConfig(fixtures.gsmBody.gsm_update, globalState, false);
      });

      context("Make payment", () => {
        context("[Failed] Make payment", () => {
          it("Payment create call", () => {
            let data =
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
            let data =
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
            let data =
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
            let data =
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
