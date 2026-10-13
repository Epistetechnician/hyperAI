# Evidence and Publication Policy

## Source classes

Sources are classified as primary, independent, institutional, community, or secondary. Each source has an exact snapshot digest, URL, access date, publication date, jurisdiction, license or terms, access class, custody mode, legal basis, custodian, acquisition timestamp, permitted uses, retention/disclosure rules, and conflict notes.

The `justai-evidence-bridge-v1` state slice gives artifacts and evaluations the same receipt shape across public and restricted lanes. Restricted bytes never enter the public repository; only content digests, bounded observations, uncertainty, nonclaims, review receipts, and disclosure decisions cross the seam. See [the bridge contract](evidence-bridge.md).

## Publication states

Evidence progresses through `Submitted`, `Quarantined`, `Corroborated`, and `IndependentlyVerified`. Any record may become `Contested` or `Withdrawn`. A withdrawn or superseded record remains hash-linked and is never silently edited.

## Verification

Publication requires two independent reviewers, conflict disclosures, provenance, an affected-party response opportunity where feasible, and a claim ceiling. A source or claim with missing custody, identity, or independent review cannot be treated as accepted evidence.

## Disclosure

Public releases contain verified facts and aggregates. Personal data, credentials, secrets, exploit details, and active incident information remain protected. The static renderer is not an adjudicator and cannot upgrade evidence status.
