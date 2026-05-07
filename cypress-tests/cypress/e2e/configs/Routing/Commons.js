export const connectorDetails = {
  priorityRouting: {
    Request: {
      name: "priority routing",
      description: "some desc",
      algorithm: {
        type: "priority",
        data: [],
      },
      profile_id: "{{profile_id}}",
    },
    Response: {
      status: 200,
      body: {},
    },
  },
  jwt: {
    Request: {},
    Response: {
      status: 200,
      body: {},
    },
  },
  volumeBasedRouting: {
    Request: {
      name: "volume routing",
      description: "some desc",
      algorithm: {
        type: "volume_split",
        data: [],
      },
      profile_id: "{{profile_id}}",
    },
    Response: {
      status: 200,
      body: {},
    },
  },
  ruleBasedRouting: {
    Request: {
      name: "Rule Based routing",
      description: "Rule Based routing",
      algorithm: {
        type: "advanced",
        data: [],
      },
      profile_id: "{{profile_id}}",
    },
    Response: {
      status: 200,
      body: {},
    },
  },
  defaultRouting: {
    Request: {
      name: "default routing",
      description: "default routing config",
      algorithm: {
        type: "priority",
        data: [],
      },
      profile_id: "{{profile_id}}",
    },
    Response: {
      status: 200,
      body: {},
    },
  },
  dynamicRouting: {
    Request: {
      decision_engine_configs: {
        defaultBucketSize: 200,
        defaultHedgingPercent: 5,
      },
    },
    Response: {
      status: 200,
      body: {},
    },
  },
  deactivateRouting: {
    Request: {
      profile_id: "{{profile_id}}",
      algorithm_for: "payment",
    },
    Response: {
      status: 200,
      body: {},
    },
  },
  deactivateRoutingNegative: {
    Request: {
      profile_id: "{{profile_id}}",
      algorithm_for: "payment",
    },
    Response: {
      status: 400,
      body: {
        error: {
          message: "Algorithm is already inactive",
          code: "IR_16",
        },
      },
    },
  },
  SurchargeDecisionManager: {
    CreateRate: {
      Request: {
        name: "surcharge_config_rate",
        merchant_surcharge_configs: {},
        algorithm: {
          type: "rate",
          rate: 2.5,
          defaultSelection: {
            surcharge_type: "rate",
            rate: 2.5,
          },
          rules: [],
        },
      },
      Response: {
        status: 200,
        body: {},
      },
    },
    RetrieveRate: {
      Request: {},
      Response: {
        status: 200,
        body: {},
      },
    },
    CreateFixed: {
      Request: {
        name: "surcharge_config_fixed",
        merchant_surcharge_configs: {},
        algorithm: {
          type: "fixed",
          amount: 100,
          defaultSelection: {
            surcharge_type: "fixed",
            amount: 100,
          },
          rules: [],
        },
      },
      Response: {
        status: 200,
        body: {},
      },
    },
    RetrieveFixed: {
      Request: {},
      Response: {
        status: 200,
        body: {},
      },
    },
    CreateConditional: {
      Request: {
        name: "surcharge_config_complex",
        merchant_surcharge_configs: {
          show_surcharge_breakup_screen: true,
        },
        algorithm: {
          type: "conditional",
          defaultSelection: {
            surcharge_type: "fixed",
            amount: 50,
          },
          rules: [
            {
              name: "Card Rule",
              connectorSelection: {
                surcharge_type: "rate",
                rate: 3.0,
              },
              conditions: [
                {
                  field: "payment_method_type",
                  operator: "equals",
                  value: "card",
                },
                {
                  field: "card_network",
                  operator: "in",
                  value: ["visa", "mastercard"],
                },
              ],
              action: {
                surcharge_type: "rate",
                rate: 3.0,
              },
            },
            {
              name: "PayPal Rule",
              connectorSelection: {
                surcharge_type: "fixed",
                amount: 200,
              },
              conditions: [
                {
                  field: "payment_method_type",
                  operator: "equals",
                  value: "paypal",
                },
              ],
              action: {
                surcharge_type: "fixed",
                amount: 200,
              },
            },
          ],
        },
      },
      Response: {
        status: 200,
        body: {},
      },
    },
    RetrieveConditional: {
      Request: {},
      Response: {
        status: 200,
        body: {},
      },
    },
    Delete: {
      Request: {},
      Response: {
        status: 200,
        body: {},
      },
    },
    RetrieveDeleted: {
      Request: {},
      Response: {
        status: 404,
        body: {},
      },
    },
  },
  deactivateDynamicRouting: {
    Request: {},
    Response: {
      status: 200,
      body: {},
    },
  },
  toggleRouting: {
    Request: {},
    Response: {
      status: 200,
      body: {
        kind: "dynamic",
      },
    },
  },
};
