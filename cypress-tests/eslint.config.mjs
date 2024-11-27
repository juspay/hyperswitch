import pluginJs from "@eslint/js";
import eslintConfigPrettier from 'eslint-config-prettier';
import pluginCypress from 'eslint-plugin-cypress/flat';
import globals from "globals";


/** @type {import('eslint').Linter.Config[]} */
export default [{
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node
      }
    }
  },
  pluginJs.configs.recommended,
  pluginCypress.configs.recommended,
  eslintConfigPrettier,
  {
    rules: {
      'prefer-const': 'warn',
      'no-console': 'warn',
      'cypress/assertion-before-screenshot': 'warn',
      'cypress/no-assigning-return-values': 'warn',
      'cypress/no-force': 'warn',
      'cypress/no-unnecessary-waiting': 'warn',
      'cypress/unsafe-to-chain-command': 'warn',
    }
  }
];