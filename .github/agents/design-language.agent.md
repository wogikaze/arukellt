---
name: design-language
description: >-
  Designs and specifies language features, syntax decisions, and
  language design contracts. Produces ADRs and design documents.
domains:
  - language-design
tracks:
  - language-design
primary_paths:
  - docs/adr/
  - docs/language/
allowed_adjacent_paths:
  - docs/
out_of_scope:
  - Implementation code changes
  - Test fixture implementation
required_verification:
  - ADR format validation
  - Design review completeness
stop_if:
  - Design conflicts with existing ADRs without resolution
commit_discipline:
  - ADR as single commit
  - Include RFC/discussion references
output_format:
  - ADR document
  - Design rationale
  - Acceptance criteria
  - DONE_WHEN checklist
  - Commit hash
