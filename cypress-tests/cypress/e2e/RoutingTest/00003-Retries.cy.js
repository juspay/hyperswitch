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

    it("Create JWT token", () => {
      let data = utils.getConnectorDetails("common")["jwt"];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createJWTToken(req_data, res_data, globalState);
    });

    it("List MCA", () => {
      cy.ListMCAbyMID(globalState);
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
  context("Auto retries", () => {
    context("[ON] Auto retries", () => {
      it("Enable auto retries", () => {
        cy.autoRetryConfig(fixtures.autoretries.gsm, globalState, "true");
      });

      context("Stripe -> Adyen auto retries", () => {
        context("Max auto retries", () => {
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
              let data = utils.getConnectorDetails("common")["routing"];
              let req_data = data["Request"];
              let res_data = data["Response"];

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
                req_data,
                res_data,
                "priority",
                routing_data,
                globalState
              );
            });

            it("Activate routing config", () => {
              let data = utils.getConnectorDetails("common")["routing"];
              let req_data = data["Request"];
              let res_data = data["Response"];
              cy.activateRoutingConfig(req_data, res_data, globalState);
            });
          });

          context("Max auto retries = 1", () => {
            const max_auto_retries = 1;
            it("Update max auto retries", () => {
              cy.setMaxAutoRetries(
                fixtures.autoretries.max_auto_retries,
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
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.createPaymentIntentTest(
                  fixtures.createPaymentBody,
                  req_data,
                  res_data,
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
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.confirmCallTest(
                  fixtures.confirmBody,
                  req_data,
                  res_data,
                  true,
                  globalState
                );
              });

              it("Payment retrieve call", () => {
                cy.retrievePaymentCallTest(
                  globalState,
                  true,
                  max_auto_retries + 1
                );
              });
            });
          });
          context("Max auto retries = 0", () => {
            const max_auto_retries = 0;
            it("Update max auto retries", () => {
              cy.setMaxAutoRetries(
                fixtures.autoretries.max_auto_retries,
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
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.createPaymentIntentTest(
                  fixtures.createPaymentBody,
                  req_data,
                  res_data,
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
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.confirmCallTest(
                  fixtures.confirmBody,
                  req_data,
                  res_data,
                  true,
                  globalState
                );
              });

              it("Payment retrieve call", () => {
                cy.retrievePaymentCallTest(
                  globalState,
                  true,
                  max_auto_retries + 1
                );
              });
            });
          });

          context("Max auto retries = 2", () => {
            const max_auto_retries = 2;
            it("Update max auto retries", () => {
              cy.setMaxAutoRetries(
                fixtures.autoretries.max_auto_retries,
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
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.createPaymentIntentTest(
                  fixtures.createPaymentBody,
                  req_data,
                  res_data,
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
                let req_data = data["Request"];
                let res_data = data["Response"];
                cy.confirmCallTest(
                  fixtures.confirmBody,
                  req_data,
                  res_data,
                  true,
                  globalState
                );
              });

              it("Payment retrieve call", () => {
                cy.retrievePaymentCallTest(
                  globalState,
                  true,
                  max_auto_retries + 1
                );
              });
            });
          });
        });
      });

      context("Adyen -> Stripe auto retries", () => {});
    });
    context("[OFF] Auto retries", () => {
      it("Disable auto retries", () => {
        cy.autoRetryConfig(fixtures.autoretries.gsm, globalState, "false");
      });
      context("[Failed] Make payment", () => {
        it("Payment create call", () => {
          let data =
            utils.getConnectorDetails("autoretries")["card_pm"][
              "PaymentIntent"
            ];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            req_data,
            res_data,
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
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.confirmCallTest(
            fixtures.confirmBody,
            req_data,
            res_data,
            true,
            globalState
          );
        });

        it("Payment retrieve call", () => {
          cy.retrievePaymentCallTest(globalState, true);
        });
      });
      context("[Success] Make payment", () => {
        it("Payment create call", () => {
          let data =
            utils.getConnectorDetails("autoretries")["card_pm"][
              "PaymentIntent"
            ];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            req_data,
            res_data,
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
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.confirmCallTest(
            fixtures.confirmBody,
            req_data,
            res_data,
            true,
            globalState
          );
        });

        it("Payment retrieve call", () => {
          cy.retrievePaymentCallTest(globalState, true);
        });
      });
    });
  });

  context("Step up 3DS", () => {
    context("[ON] Step up", () => {});
    context("[OFF] Step up", () => {});
  });
});
