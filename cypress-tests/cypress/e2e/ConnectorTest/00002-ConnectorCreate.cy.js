
// import createConnectorBody from "../../fixtures/create-connector-body.json";
// import ConnectorAuthDetails from "../../../creds.json";


// describe("Connector Account Create flow test", () => {
//   it("connector-create-call-test", () => {
//     cy.createMerchantConnectorTest(createConnectorBody, "stripe");
//   });
  
// });

// Cypress.Commands.add("createMerchantConnectorTest", (connectorBody, authType) => {
//   let x = getValueByKey(ConnectorAuthDetails, authType);
//   console.log("abcdx"+x);
//     // Check if the authType exists in the creds.json file
//     if (x) {
//       // Use the specified authType details from creds.json
//       const authDetails = ConnectorAuthDetails[x];
//       console.log("authDetails"+authDetails);
//       describe("Account Create flow test", () => {
//         it("connector-create-call-test", () => {
//           cy.createConnectorCallTest(connectorBody, authDetails);
//         });
        
//       });
  
//     } else {
//       // Handle the case where the specified authType is not found
//       throw new Error(`Authentication details not found for ${authType}`);
//     }
// });

// function getValueByKey(jsonObject, key) {
//   // Convert the input JSON string to a JavaScript object if it's a string
//   const data = typeof jsonObject === 'string' ? JSON.parse(jsonObject) : jsonObject;

//   // Check if the key exists in the object
//   if (data && typeof data === 'object' && key in data) {
//     return data[key];
//   } else {
//     return null; // Key not found
//   }
// }






import createConnectorBody from "../../fixtures/create-connector-body.json";
import State from "../../utils/State";
// import ConnectorAuthDetails from "../../../creds.json";
let globalState;
describe("Connector Account Create flow test", () => {

  before("seed global state",  () => {
    
    cy.task('getGlobalState').then((state) => {
      // visit non same-origin url https://www.cypress-dx.com
      globalState = new State(state);
      console.log("seeding globalState -> " + JSON.stringify(globalState));
    })
  })

  after("flush global state", () => {
    console.log("flushing globalState -> "+ JSON.stringify(globalState));
    cy.task('setGlobalState', globalState.data);
  })

  it("connector-create-call-test", () => {
    // Provide the authType directly, no need to use getValueByKey
    // const authType = "nmi";
    // const authDetails = getValueByKey(ConnectorAuthDetails, authType);

    // Check if the authType exists in the creds.json file
    // if (authDetails) {
      // Use the specified authType details from creds.json
      cy.createConnectorCallTest(createConnectorBody, globalState);
    // } else {
    //   // Handle the case where the specified authType is not found
    //   throw new Error(`Authentication details not found for ${authType}`);
    // }
  });
});

// function getValueByKey(jsonObject, key) {
//   // Convert the input JSON string to a JavaScript object if it's a string
//   const data = typeof jsonObject === 'string' ? JSON.parse(jsonObject) : jsonObject;

//   // Check if the key exists in the object
//   if (data && typeof data === 'object' && key in data) {
//     return data[key];
//   } else {
//     return null; // Key not found
//   }
// }
