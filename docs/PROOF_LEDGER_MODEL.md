# Proof Ledger Model

The Proof Ledger is a structured record of completed learning, practice, and participation inside Karyra Spark.

It is called a “ledger” because it keeps an ordered history of meaningful activity. It does not need to be onchain from the beginning. The first version can be stored in the application database and later connected to stronger verification systems when needed.

## Purpose

The Proof Ledger helps answer:

- What did the user complete?
- When did it happen?
- Who or what verified it?
- What readiness meaning does it carry?
- Can this record be trusted for the next step?

The Proof Ledger is not a leaderboard. It is not a trading record. It is not a public social feed.

It is a structured progress history.

## What a proof record means

A proof record represents one verified activity.

Examples:

- a lesson was completed;
- a lab was practiced;
- a quiz checkpoint was passed;
- a workshop was attended;
- a facilitator confirmed participation;
- a user became ready for a next learning stage.

Each proof record should be clear enough for both humans and systems to understand.

## Suggested record structure

```json
{
  "id": "proof_01",
  "user_id": "user_01",
  "type": "lesson_completed",
  "subject": "wallet-safety-basics",
  "source": "system",
  "status": "valid",
  "issued_at": "2026-06-10T10:00:00Z",
  "metadata": {
    "level": "core",
    "readiness_area": "wallet_safety"
  }
}
```

## Core fields

| Field | Description |
|---|---|
| `id` | Unique proof record ID |
| `user_id` | User that owns the record |
| `type` | Type of participation or completion |
| `subject` | Lesson, lab, workshop, or activity name |
| `source` | System, facilitator, import, or future attestation |
| `status` | Current validity of the record |
| `issued_at` | Time when the record was created |
| `metadata` | Additional safe context |

## Proof types

### Learning proof

Created when a user completes a structured lesson.

```text
lesson_started
lesson_completed
checkpoint_passed
```

### Practice proof

Created when a user completes a guided lab or simulation.

```text
lab_started
lab_completed
wallet_safety_practiced
signature_risk_reviewed
```

### Community proof

Created when a user joins a workshop, cohort, or local learning activity.

```text
workshop_registered
workshop_attended
community_session_completed
```

### Facilitator proof

Created when a facilitator confirms a user’s participation or readiness.

```text
facilitator_confirmed
readiness_reviewed
cohort_participation_verified
```

## Verification status

A proof record should have a status.

| Status | Meaning |
|---|---|
| `valid` | Record is active and usable |
| `pending` | Waiting for confirmation |
| `revoked` | No longer valid |
| `superseded` | Replaced by a newer record |

This helps the system avoid treating all records as permanent or equal.

## Trust levels

Not all proof records have the same strength.

| Trust level | Example |
|---|---|
| Basic | User opened or started a lesson |
| System-verified | User completed a lesson or lab checkpoint |
| Facilitator-verified | A facilitator confirmed workshop participation |
| Future attested | A record is linked to a stronger external proof |

The user interface should explain these differences in simple language.

## Relationship to the Readiness Passport

The Proof Ledger stores the raw records.

The Readiness Passport summarizes them.

For example, the Proof Ledger may contain many records:

```text
lesson_completed: blockchain basics
lesson_completed: wallet safety
lab_completed: read before signing
workshop_attended: local Starknet intro
```

The Readiness Passport may summarize them as:

```text
Core readiness: completed
Wallet safety: practiced
Community participation: verified
Suggested next step: beginner Starknet exploration
```

## Privacy and sharing

Proof records should be private by default.

A user may later choose to share selected summaries or proofs, but the system should not assume that every record is public.

Recommended design:

- keep detailed records private;
- show readable summaries to the user;
- allow facilitator access only where needed;
- make public sharing explicit;
- avoid exposing unnecessary personal data.

## Future onchain use

The Proof Ledger can later support Starknet-based attestations.

A future version may allow selected records to become shareable proofs. This should be optional and should not require exposing the full private learning history.

Possible future flow:

```text
Private proof record
        ↓
User chooses to share
        ↓
System prepares limited proof data
        ↓
Proof is linked to an attestation
        ↓
User can present the proof outside Spark
```

The first version should remain simple: database-backed, clear, auditable, and easy to understand.
