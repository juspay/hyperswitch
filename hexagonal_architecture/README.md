Hexagonal architecture is a software design pattern that helps to separate the business logic from the input/output logic. It does this by defining interfaces for each external interaction, which can then be injected into services that use them. This allows for testing with mocks and interchangeable implementations.

Ports and Adapters is an architectural pattern that helps to separate the business logic from technical implementation details. It does this by creating ports, which are the boundaries of an application, and adapters, which are used to translate between the ports and the application model. This allows us to be more flexible and respond quickly to changes in business or technology.

Ports:
* Connector
* Payments

Adapters:
* akita_adapter
* memory_adapter
