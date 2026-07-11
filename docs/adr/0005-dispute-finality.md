# ADR 0005: dispute and finality

Status: accepted for implementation.

Truth, finality, and settlement lifecycle are separate state machines. Active
disputes and operator pauses block finalization and settlement. Corrections are
signed superseding records; they never silently replace a final resolution or
drive the original binding.
