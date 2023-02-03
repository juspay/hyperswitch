# Postman Collection

You can find the latest Postman collection [here][postman-collection].
For getting started quickly, you can also
[fork the Postman collection][postman-collection-fork] under your workspace.

## Auto-Generating the Postman Collection

The [Postman collection][postman-collection] is generated using
[`portman`][portman] tool from the [OpenAPI specification][openapi-spec].
If you'd like to generate the collection from the OpenAPI specification, you can
follow the below steps:

1. Install `portman`, refer to the instructions on
   [the repository][portman-repository]:

   ```shell
   npm install -g @apideck/portman
   ```

2. From the root of the project directory, run the following command to generate
   the Postman collection.

   ```shell
   portman --cliOptionsFile postman/portman-cli.json
   ```

**NOTE:** Please verify Postman collection variables before trying out the APIs.

[postman-collection]: https://www.postman.com/hyperswitch/workspace/hyperswitch/collection/25176183-e36f8e3d-078c-4067-a273-f456b6b724ed
[postman-collection-fork]: https://www.postman.com/hyperswitch/workspace/hyperswitch/collection/25176183-e36f8e3d-078c-4067-a273-f456b6b724ed/fork
[portman]: https://www.npmjs.com/package/@apideck/portman
[openapi-spec]: /openapi/open_api_spec.yaml
[portman-repository]: https://github.com/apideck-libraries/portman
