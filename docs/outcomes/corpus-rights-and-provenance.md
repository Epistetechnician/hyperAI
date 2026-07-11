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

The current build validates profiles and pins and can materialize regular bytes
from a caller-provided checkout only after exact commit and path checks. It does
not clone, authenticate, resolve special Git objects, or change the recorded
rights ceiling. Therefore no external corpus is claimed as a live completeness
oracle or settlement-ready source.
