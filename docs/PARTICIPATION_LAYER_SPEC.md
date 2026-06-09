# Participation Layer

The Participation Layer is the part of Karyra Spark that records meaningful learning and community activity in a structured way.

It is designed to answer a simple question:

> What has a learner actually completed, practiced, or joined?

Instead of treating learning as a single score, the Participation Layer separates activity into clear records. These records can come from lessons, labs, workshops, community sessions, facilitator reviews, and future ecosystem tasks.

The goal is not to rank users. The goal is to help learners, facilitators, and applications understand readiness in a safer and more transparent way.

## Why this layer exists

Many new users enter Web3 without a clear sense of readiness. They may connect a wallet, sign a message, or follow a transaction flow before they understand the risks.

Karyra Spark takes a different path.

Users begin with basic concepts, safety habits, practice flows, and guided learning. The Participation Layer helps record this process so that progress is not only remembered by the user interface, but also represented as structured data.

This makes it possible to build features such as:

- lesson completion history;
- lab practice records;
- workshop attendance records;
- facilitator-verified participation;
- readiness summaries;
- passport-style progress views;
- future attestations or proofs.

## Core principles

### 1. Learning first

The system records learning activity before any wallet-based action is required.

A user should be able to understand concepts, risks, and basic flows without needing to connect a wallet or perform a transaction.

### 2. Safety before proof

Not every activity needs to become a public proof.

Some records may stay private to the user account. Some may be visible only to facilitators. Some may later become shareable proofs if the user chooses to do so.

### 3. Human-readable records

Participation records should be understandable by non-technical users.

A learner should be able to read a record and understand what it means, for example:

- completed a beginner lesson;
- practiced a wallet safety scenario;
- joined a local workshop;
- received facilitator confirmation;
- became ready for the next learning stage.

### 4. Verifiable when needed

Some records can be system-generated, such as completing a lesson or passing a lab checkpoint.

Other records may require a facilitator, such as confirming workshop attendance or community participation.

The system should make the source of each record clear.

### 5. User control

Participation records should not expose unnecessary personal information.

Future public proofs or attestations should be designed around consent, minimization, and clear user understanding.

## Types of participation

### Lesson participation

A lesson record represents structured learning progress.

```json
{
  "type": "lesson_completed",
  "lesson": "wallet-safety-basics",
  "level": "core",
  "verified_by": "system"
}
```

### Lab participation

A lab record represents safe practice.

```json
{
  "type": "lab_completed",
  "lab": "read-before-signing",
  "risk_area": "wallet_signature",
  "verified_by": "system"
}
```

### Workshop participation

A workshop record represents attendance or activity in a community session.

```json
{
  "type": "workshop_attended",
  "workshop": "intro-to-starknet-safety",
  "verified_by": "facilitator"
}
```

### Facilitator review

A facilitator review adds human confirmation to a participation record.

It may be used when a user completes an offline activity, joins a local session, helps another learner, or participates in a guided community program.

## Basic flow

```text
User learns or participates
        ↓
System or facilitator checks the activity
        ↓
A participation record is created
        ↓
The record becomes part of the user's progress history
        ↓
The Readiness Passport summarizes the user's current stage
```

## Verification sources

A participation record can come from different sources.

| Source | Meaning |
|---|---|
| System | Created automatically by the application |
| Facilitator | Confirmed by a trusted human facilitator |
| Import | Added from an approved external source |
| Future attestation | Linked to a future verifiable proof |

## Privacy model

The Participation Layer should avoid storing more data than needed.

A record should focus on the activity, verification source, timestamp, and readiness meaning. It should not expose sensitive personal data unless the user clearly chooses to share it.

Recommended visibility levels:

| Visibility | Description |
|---|---|
| Private | Visible only to the user |
| Facilitator | Visible to approved facilitators |
| Community | Visible in limited community context |
| Public | Shareable by user choice |

## Future Starknet direction

In the future, selected participation records may be connected to Starknet-based attestations or proofs.

This does not mean every learning action must be placed onchain.

A careful design should separate:

- private learning history;
- application-level verification;
- facilitator confirmation;
- public proof;
- optional onchain attestation.

This allows Karyra Spark to support Web3 readiness without forcing users into public exposure too early.

## Status

The Participation Layer is a design foundation for Karyra Spark.

The first implementation should focus on simple, reliable, database-backed records. More advanced proof and attestation features can be added gradually after the user experience, safety model, and facilitator workflow are stable.
