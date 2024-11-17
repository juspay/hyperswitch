import pluginJs from "@eslint/js";
import globals from "globals";

/** @type {import('eslint').Linter.Config[]} */
export default [
  {
    languageOptions: {
      globals: globals.browser,
    },
    env: {
      es6: true,
      browser: true,
      es2021: true,
    },
    extends: ["prettier"],
    parserOptions: {
      ecmaVersion: 12,
      sourceType: "module",
    },
    rules: {
      "prettier/prettier": "error",
    },
    plugins: ["prettier"],
  },
  pluginJs.configs.recommended,
];
