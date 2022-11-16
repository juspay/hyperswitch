## POSTMAN Collection

The [Postman](https://www.postman.com/) [collection](./collection.postman.json) is generated using [Portman](https://www.npmjs.com/package/@apideck/portman) tool from the [OpenApi Spec](../openapi/open_api_spec.yaml).

Steps to generate the new collection.

* Install portman [ [refer to github](https://github.com/apideck-libraries/portman) ]

    ```bash
    # Global install
    $ npm install -g @apideck/portman
    ```

* From the base directory, run

    ```bash
    portman --cliOptionsFile postman/portman-cli.json
    ```