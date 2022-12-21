Hexagonal architecture is a software design pattern that helps to separate the business logic from the input/output logic. It does this by defining interfaces for each external interaction, which can then be injected into services that use them. This allows for testing with mocks and interchangeable implementations.

Ports and Adapters is an architectural pattern that helps to separate the business logic from technical implementation details. It does this by creating ports, which are the boundaries of an application, and adapters, which are used to translate between the ports and the application model. This allows us to be more flexible and respond quickly to changes in business or technology.

App does not have unit tests, but it does have acceptance tests. Unit tests are tests that are written to test individual components of a program or system. Acceptance tests are tests written to determine if the program or system meets the requirements of the customer or user. Acceptance tests are usually done after unit tests have been completed to make sure that the requirements have been met.

Ports:
* Connector
* Payments

Adapters:
* Stripe
* akita_adapter
* memory_adapter
