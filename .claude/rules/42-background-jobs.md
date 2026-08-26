---
paths:
  - "{workers/**,functions/scheduled/**,functions/events/**,services/**/jobs/**}"
---

# Background Work

Workers and scheduled jobs must make retry, idempotency, ordering, dead-letter,
and observability behavior explicit. Prefer small handlers around reusable
application services rather than embedding domain logic in queue glue.
