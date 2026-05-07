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
        merchantSurchargeConfigs: {},
        algorithm: {
          defaultSelection: {
            surchargeDetails: {
              surcharge: {
                type: "rate",
                value: {
                  percentage: 2.5,
                },
              },
            },
          },
          rules: [],
          metadata: {},
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
        merchantSurchargeConfigs: {},
        algorithm: {
          defaultSelection: {
            surchargeDetails: {
              surcharge: {
                type: "fixed",
                value: {
                  amount: 100,
                },
              },
            },
          },
          rules: [],
          metadata: {},
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
        merchantSurchargeConfigs: {
          showSurchargeBreakupScreen: true,
        },
        algorithm: {
          defaultSelection: {
            surchargeDetails: {
              surcharge: {
                type: "rate",
                value: {
                  percentage: 2.5,
                },
              },
            },
          },
          rules: [
            {
              name: "Card Rule",
              connectorSelection: {
                surchargeDetails: {
                  surcharge: {
                    type: "rate",
                    value: {
                      percentage: 3.0,
                    },
                  },
                },
              },
              statements: [
                {
                  condition: [
                    {
                      lhs: "payment_method",
                      comparison: "equal",
                      value: {
                        type: "enum_variant",
                        value: "card",
                      },
                      metadata: {},
                    },
                  ],
                  nested: null,
                },
              ],
            },
            {
              name: "PayPal Rule",
              connectorSelection: {
                surchargeDetails: {
                  surcharge: {
                    type: "fixed",
                    value: {
                      amount: 200,
                    },
                  },
                },
              },
              statements: [
                {
                  condition: [
                    {
                      lhs: "payment_method",
                      comparison: "equal",
                      value: {
                        type: "enum_variant",
                        value: "paypal",
                      },
                      metadata: {},
                    },
                  ],
                  nested: null,
                },
              ],
            },
          ],
          metadata: {
            description:
              "Complex surcharge with payment method conditions",
          },
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
