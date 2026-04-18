# Orchestration run — extended agent registry

This file records **named agents available for the current orchestration run** in addition to the parent prompt’s default list.

## Prompt-default agents (baseline)

- `impl-selfhost`
- `impl-stdlib`
- `impl-playground`
- `impl-runtime`
- `impl-compiler`
- `impl-vscode-ide`
- `impl-cli`
- `impl-language-docs`
- `impl-selfhost-retirement`
- `impl-component-model`
- `impl-editor-runtime`

## This run — additionally registered (repo specs)

| Agent | Spec path |
|-------|-----------|
| `impl-benchmark` | `.github/agents/impl-benchmark.agent.md` |
| `impl-verification-infra` | `.github/agents/impl-verification-infra.agent.md` |

Parent treats these as **dispatchable** for benchmark / verification-infra tracks without creating duplicate agent specs.
