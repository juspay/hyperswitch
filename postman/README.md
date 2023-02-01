## POSTMAN Collection

The [Postman](https://www.postman.com/) [collection](./collection.postman.json) is generated using [Portman](https://www.npmjs.com/package/@apideck/portman) tool from the [OpenApi Spec](../openapi/open_api_spec.yaml).

Steps to generate the new collection.

- Get latest Postman Collection from [here](https://www.postman.com/hyperswitch/workspace/hyperswitch/collection/25176183-e36f8e3d-078c-4067-a273-f456b6b724ed).\

    ```url
    https://www.postman.com/hyperswitch/workspace/hyperswitch/collection/25176183-e36f8e3d-078c-4067-a273-f456b6b724ed
    ```

* Install portman [ [refer to github](https://github.com/apideck-libraries/portman) ]

    ```bash
    # Global install
    $ npm install -g @apideck/portman
    ```

* From the base directory, run

    ```bash
    portman --cliOptionsFile postman/portman-cli.json
    ```

Note :- Please verify postman collection variables before trying out api's.