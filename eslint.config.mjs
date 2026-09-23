import js from "@eslint/js";
import globals from "globals";
import hooks from "eslint-plugin-react-hooks";
import ts from "typescript-eslint";

export default ts.config(
  {
    ignores: [
      "api-client/src/handlers/**",
      "api-client/src/types/**",
      "plugin-dist/**",
      "dist/**",
      "api/**",
      "ui-kit/**",
    ],
  },
  js.configs.recommended,
  ...ts.configs.recommended,
  {
    files: ["**/*.{ts,tsx,mjs}"],
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
    rules: { "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }] },
  },
  {
    files: ["plugins/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            "**/web/**",
            "**/api-client/**",
            "@interview/api-client",
            "@interview/api-client/*",
            "!**/api-client/src/types/**",
          ],
        },
      ],
    },
  },
  {
    files: ["web/src/**/*.{ts,tsx}", "plugins/**/*.{ts,tsx}", "plugin-sdk/**/*.{ts,tsx}"],
    plugins: { "react-hooks": hooks },
    rules: hooks.configs.recommended.rules,
  },
);
