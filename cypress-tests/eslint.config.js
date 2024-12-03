import pluginJs from "@eslint/js";
import eslintConfigPrettier from "eslint-config-prettier";
import pluginCypress from "eslint-plugin-cypress/flat";
import eslintPluginPrettierRecommended from "eslint-plugin-prettier/recommended";
import globals from "globals";

/** @type {import('eslint').Linter.Config[]} */
export default [
  pluginJs.configs.recommended,
  pluginCypress.configs.recommended,
  eslintPluginPrettierRecommended,
  eslintConfigPrettier,
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    rules: {
      "no-unused-vars": "error",
      "no-undef": "error",
      "no-console": "warn",
      "prefer-const": "warn",

      "cypress/assertion-before-screenshot": "warn",
      "cypress/no-assigning-return-values": "warn",
      "cypress/no-force": "warn",
      "cypress/no-unnecessary-waiting": "warn",
      "cypress/no-async-tests": "error",
      "cypress/unsafe-to-chain-command": "warn",

      "prettier/prettier": "error",
    },
  },
];
