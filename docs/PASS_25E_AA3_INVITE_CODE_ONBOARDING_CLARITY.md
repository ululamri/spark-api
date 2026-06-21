# PASS 25E-AA3 — Invite Code Onboarding Clarity

This pass fixes an essential invite onboarding UX/security gap.

## Problem

The backend used a raw invitation token internally, and the first onboarding screen used the term `Invite token`.
The email invitation contained the onboarding URL but did not expose the code separately as a human-facing invite code.

## Fix

- The invite email now includes a visible `Kode undangan`.
- The invite email still includes the onboarding link.
- The onboarding link keeps using `?token=...` internally for API compatibility.
- The frontend onboarding page now reads `?token=` and pre-fills it as `Invite code`.
- User-facing copy uses `Invite code`, not `Invite token`.

## Boundary

The raw invite code is still stored only as a hash in the database.
The raw code is included only in the invite email and onboarding URL sent to the invited address.
