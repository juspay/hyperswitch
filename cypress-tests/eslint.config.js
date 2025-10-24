import pluginJs from "@eslint/js";
import eslintConfigPrettier from "eslint-config-prettier";
import pluginCypress from "eslint-plugin-cypress/flat";
import eslintPluginPrettierRecommended from "eslint-plugin-prettier/recommended";
import globals from "globals";
import tseslint from "typescript-eslint";

/** @type {import('eslint').Linter.Config[]} */
export default [
  pluginJs.configs.recommended,
  pluginCypress.configs.recommended,
  eslintPluginPrettierRecommended,
  eslintConfigPrettier,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.ts", "**/*.tsx"],
    rules: {
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_.*$" },
      ],
      "@typescript-eslint/explicit-module-boundary-types": "off",
      "@typescript-eslint/ban-ts-comment": "off",
      "@typescript-eslint/no-non-null-assertion": "off",
    },
  },
  {
    ignores: ["dist/**", "build/**", "node_modules/**"],
  },
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    rules: {
      "no-unused-vars": ["error", { argsIgnorePattern: "^_.*$" }],
      "no-undef": "error",
      "no-console": "warn",
      "prefer-const": "warn",
      "cypress/assertion-before-screenshot": "warn",
      "cypress/no-assigning-return-values": "warn",
      "@typescript-eslint/no-unused-expressions": "off",
      "cypress/no-force": "warn",
      "cypress/no-unnecessary-waiting": "warn",
      "cypress/no-async-tests": "error",
      "cypress/unsafe-to-chain-command": "warn",
      "prettier/prettier": "error",
    },
  },
];
