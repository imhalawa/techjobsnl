# Release prototypes

Every unshipped release keeps its prototype plan and review evidence in its private release Project draft item. Draft task
decomposition may identify prototype questions and journeys early, but applicable prototypes must be accepted before a task
becomes Ready. A prototype validates the complete promised journey, major states, terminology, information hierarchy, and
design-system fit. Shipping publishes only the safe approval record at `releases/<version>/prototype/README.md`.

The prototype record includes its artifact location, realistic synthetic data, required journeys and states, screenshots or
recordings, review decisions, rejected alternatives, approval, and unresolved implementation constraints. It contains no
credentials, personal data, licensed source payloads, or confidential third-party material.

Prototype fidelity should answer the release's uncertain interaction questions and stop there. Production architecture,
persistence, and provider behavior remain proven by implementation tasks and tests.

The GitHub repository is public. Keep confidential prototype artifacts locally or in an access-controlled design workspace;
mirror their private planning and review record only as Project draft items under `docs/PLANNING_PRIVACY.md`. Never place a
confidential artifact or its substantive design content in a public commit, pull request, or repository issue.
