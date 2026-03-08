import js from "@eslint/js";
import importX from "eslint-plugin-import-x";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "**/node_modules/**",
      "**/dist/**",
      "**/coverage/**",
      "**/target/**",
      "**/.turbo/**",
      "**/.volt-e2e-smoke-*/**",
      "**/*.d.ts",
      "crates/volt-napi/*.node",
    ],
  },
  {
    files: ["**/*.{js,mjs,cjs,ts,mts,cts}"],
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: {
        ...globals.node,
        ...globals.browser,
      },
    },
    plugins: {
      "import-x": importX,
    },
  },
  {
    files: ["**/*.{js,mjs,cjs}"],
    rules: {
      "import-x/no-unresolved": ["error", { ignore: ["^volt:"] }],
    },
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{ts,mts,cts}"],
    rules: {
      "@typescript-eslint/no-explicit-any": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
        },
      ],
    },
  },
  {
    files: [
      "**/*.test.{js,mjs,cjs,ts,mts,cts}",
      "**/__tests__/**/*.{js,mjs,cjs,ts,mts,cts}",
    ],
    languageOptions: {
      globals: {
        ...globals.vitest,
      },
    },
    rules: {
      "import-x/no-unresolved": "off",
    },
  },
  {
    files: ["packages/create-volt/src/templates/**/*.{js,mjs,cjs,ts,mts,cts}"],
    rules: {
      "import-x/no-unresolved": "off",
    },
  },
  {
    files: ["crates/volt-napi/index.js"],
    languageOptions: {
      sourceType: "commonjs",
    },
  },
);
