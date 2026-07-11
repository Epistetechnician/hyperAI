# Corpus rights and provenance

`config/corpus-profiles.json` is the machine-readable inventory. Every profile
pins a source URL and immutable commit, separates cataloging from redistribution,
and reports a readiness ceiling.

Public Git availability is not redistribution permission. A repository-wide
license is only one provenance fact; nested event/challenge artifacts require
item-level review. Missing or ambiguous rights force `CATALOG_ONLY`. Hosted
services are discovery-only unless an official API and explicit authorization
support the intended use. The 247CTF profile is manual/recorded only and performs
no authentication or hosted interaction.

The current build validates profiles and pins but does not materialize upstream
challenge bytes. Therefore none of the external corpus profiles is claimed as a
live submission completeness oracle or settlement-ready source.
