// Conventional Commits configuration for qiqi.
// Types are restricted to the ones used by this project's workflow.
export default {
  extends: ["@commitlint/config-conventional"],
  parserPreset: {
    parserOpts: {
      issuePrefixes: ["#"],
    },
  },
  rules: {
    "type-enum": [
      2,
      "always",
      [
        "feat",
        "fix",
        "refactor",
        "perf",
        "style",
        "docs",
        "test",
        "build",
        "ci",
        "chore",
        "revert",
      ],
    ],
    // Scopes map to the app's modules; extend as the code grows.
    "scope-enum": [
      2,
      "always",
      [
        "renderer",
        "ui",
        "ipc",
        "window",
        "db",
        "ai",
        "memory",
        "affection",
        "probe",
        "deps",
        "docs",
        "ads",
        "pet-design",
        "website",
      ],
    ],
    "subject-case": [2, "always", ["lower-case", "sentence-case"]],
    "subject-empty": [2, "never"],
    "subject-max-length": [2, "always", 100],
    "header-max-length": [2, "always", 120],
    "body-max-line-length": [0, "always", 120],
  },
};
