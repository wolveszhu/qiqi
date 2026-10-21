import js from "@eslint/js";
import pluginVue from "eslint-plugin-vue";
import vueParser from "vue-eslint-parser";
import tseslint from "typescript-eslint";

// ESLint flat config for qiqi (Vue 3 + TypeScript + Vite).
// ESLint handles CORRECTNESS only; formatting is delegated to Prettier.
export default tseslint.config(
  {
    ignores: [
      "dist/**",
      "node_modules/**",
      "public/**",
      "docs/**",
      "src-tauri/**", // Rust side is linted by clippy/rustfmt
    ],
  },

  js.configs.recommended,
  ...pluginVue.configs["flat/recommended"],
  ...tseslint.configs.recommended,

  // Let <script lang="ts"> blocks inside .vue be parsed as TypeScript.
  // Explicitly wire the SFC parser: under ESLint 10 + eslint-plugin-vue 10,
  // the flat/recommended preset alone did not apply it (App.vue parse errors).
  {
    files: ["**/*.vue"],
    languageOptions: {
      parser: vueParser,
      parserOptions: { parser: tseslint.parser },
    },
  },

  {
    files: ["**/*.{ts,tsx,vue}"],
    rules: {
      // TypeScript already resolves identifiers; core no-undef produces
      // false positives on browser globals (window/document).
      "no-undef": "off",

      // --- Core correctness ---
      "no-console": ["warn", { allow: ["warn", "error"] }],
      "no-var": "error",
      "prefer-const": "error",
      eqeqeq: ["error", "always", { null: "ignore" }],
      "no-param-reassign": ["error", { props: true }],

      // --- TypeScript ---
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/consistent-type-imports": ["error", { prefer: "type-imports" }],
      "@typescript-eslint/explicit-module-boundary-types": "warn",
      "@typescript-eslint/no-non-null-assertion": "warn",
      "@typescript-eslint/no-unused-vars": ["warn", { argsIgnorePattern: "^_" }],

      // --- Vue 3 ---
      "vue/component-api-style": ["error", ["script-setup"]],
      "vue/component-name-in-template-casing": ["error", "PascalCase"],
      "vue/no-mutating-props": "error",
      "vue/no-v-html": "error",
      "vue/require-explicit-emits": "warn",
      "vue/html-indent": ["warn", 2],
      // Align with Prettier (formatting is delegated to Prettier, see header):
      // Prettier emits self-closing void elements and keeps short tags single-line.
      "vue/html-self-closing": [
        "warn",
        { html: { void: "always", normal: "always", component: "always" } },
      ],
      "vue/singleline-html-element-content-newline": "off",

      // --- Explicitly disabled (documented) ---
      // Off: root App.vue is a single-word component name by convention.
      "vue/multi-word-component-names": "off",
      // Off: TS type-based props make default declarations redundant.
      "vue/require-default-prop": "off",
    },
  },

  // Tooling/build files may use looser patterns.
  {
    files: ["*.config.js", "*.config.ts", "vite.config.ts"],
    rules: {
      "@typescript-eslint/no-explicit-any": "off",
      "no-console": "off",
    },
  },
);
