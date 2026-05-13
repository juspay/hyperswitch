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
    Create: {
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
        body: {
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
      },
    },
    Retrieve: {
      Request: {},
      Response: {
        status: 200,
        body: {
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
      },
    },
    Delete: {
      Request: {},
      Response: {
        status: 200,
        body: {
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
      },
    },
  },
  routingEvaluate: {
    Request: {
      name: "routing evaluate",
      description: "routing evaluate test config",
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
