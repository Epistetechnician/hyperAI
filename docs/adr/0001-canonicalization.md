# ADR 0001: canonical claim representation

Status: accepted for implementation.

Use RFC 8785 JSON Canonicalization Scheme for resolution-critical JSON, ban
floating-point fields, normalize protocol timestamps to UTC with fixed
precision, and apply explicit digest domain separation. Derived IDs are never
accepted as semantic input. Golden vectors must be independently reproducible.
