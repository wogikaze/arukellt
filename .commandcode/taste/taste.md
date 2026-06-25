# workflow

- Always commit changes after completing a work unit before moving to the next task; verify quick must pass before every commit. Confidence: 0.90
- Work in 1-issue-1-commit units; never mix multiple issues in a single commit. Confidence: 0.85
- Fan-out independent work to parallel agents when issues do not conflict. Confidence: 0.80
- Do not false-close issues: verify must pass with implementation-backed evidence before moving to issues/done/. Confidence: 0.85
- Always use implementation plans: read issue → plan → implement → verify → commit → continue. Confidence: 0.80

# language

- Output summaries and reports in Japanese when the user communicates in Japanese. Confidence: 0.85
- Mix Japanese and English naturally in responses; the user switches between both freely. Confidence: 0.85

# code-quality

- Fixpoint (sha256(s2)==sha256(s3)) must be stable before advancing to fixture or diagnostic parity work. Confidence: 0.80
- Preserve existing CLI contracts, fixture semantics, diagnostics schema, and selfhost gates when making changes. Confidence: 0.75
- Read and strictly follow plan documents (docs/*.md, issues/*.md) during implementation; verify progress against acceptance checklists in those documents. Confidence: 0.80
- Add verify commands/tests to acceptance checklists as proof that each requirement is met before marking done. Confidence: 0.70
