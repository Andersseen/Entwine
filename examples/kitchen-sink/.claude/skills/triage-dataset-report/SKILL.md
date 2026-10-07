---
name: triage-dataset-report
description: >
  Sort an incoming dataset problem report into duplicate, data error, or
  access question, and say what to do next.
---

# Triage a dataset report

Use the [checklist](references/checklist.md) to classify the report. A finished
example is in [examples/report.md](examples/report.md).

1. Confirm the dataset exists in the catalogue (see the
   [data model](../../../docs/design/data-model.md)).
2. Decide whether it is a duplicate, a data error, or an access question.
3. For access questions, answer from the
   [authentication spec](../../../docs/specs/authentication.md); never promise
   write access, which Harbor does not offer.
4. Record the outcome where the [runbook](../../../docs/operations/runbook.md) says.
