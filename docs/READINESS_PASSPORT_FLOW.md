# Readiness Passport Flow

The Readiness Passport is a user-facing summary of learning progress, practice history, and participation readiness in Karyra Spark.

It helps users understand where they are, what they have completed, and what they should do next.

The passport is not a financial profile. It is not a wallet score. It is not a public identity by default.

It is a readiness guide.

## Purpose

New users often need a clear path before entering Web3 applications.

The Readiness Passport helps by showing:

- completed learning stages;
- safety areas already practiced;
- unfinished readiness areas;
- community participation records;
- suggested next steps;
- optional proof or sharing status.

The goal is to make progress understandable, not to pressure users.

## Basic flow

```text
User completes lessons
        ↓
User practices labs
        ↓
User joins community activities
        ↓
System and facilitators create records
        ↓
Passport summarizes readiness
        ↓
User receives safer next steps
```

## Passport sections

### 1. Learning readiness

Shows progress through structured lessons.

```text
Core lessons: completed
Blockchain basics: completed
Wallet safety: completed
Starknet introduction: in progress
```

### 2. Safety readiness

Shows whether the user has practiced important safety concepts.

```text
Seed phrase safety: understood
Signature risk: practiced
Fake link awareness: reviewed
Mainnet caution: not yet completed
```

### 3. Practice readiness

Shows completed labs or simulations.

```text
Read-before-signing lab: completed
Wallet connect simulation: completed
Testnet orientation: in progress
```

### 4. Community participation

Shows workshop or cohort involvement.

```text
Local intro session: attended
Facilitator review: verified
Community mapping activity: pending
```

### 5. Suggested next step

The passport should recommend one or two simple next steps.

Examples:

```text
Continue Starknet basics.
Practice wallet signature safety again.
Join a beginner workshop.
Explore Hub resources marked as beginner-friendly.
```

## Readiness levels

The passport can use clear readiness levels.

| Level | Meaning |
|---|---|
| Starting | User has begun learning |
| Learning | User is completing core concepts |
| Practicing | User is using safe simulations or labs |
| Ready to explore | User can open guided ecosystem resources |
| Builder later | User may begin technical learning paths |

These levels should not be treated as permanent labels. A user can continue learning, repeat labs, or improve readiness over time.

## How the passport uses records

The passport does not need to store every detail itself.

It reads from the Proof Ledger and turns many records into a simple summary.

Example:

```text
Proof Ledger:
- lesson_completed: wallet safety
- lab_completed: read before signing
- workshop_attended: intro session

Readiness Passport:
- Wallet safety: practiced
- Community intro: verified
- Recommended next step: Starknet beginner resources
```

## Visibility

The passport should be private by default.

Users should be able to view their own progress without automatically publishing it.

Possible visibility modes:

| Mode | Description |
|---|---|
| Private | Only the user can see it |
| Facilitator view | Shared with an approved facilitator |
| Shareable summary | User chooses to share a limited version |
| Future proof | Selected records may become verifiable externally |

## What should not be shown

The passport should avoid unnecessary sensitive information.

It should not show:

- private keys;
- seed phrases;
- wallet balances;
- personal identity documents;
- full private learning history in public mode;
- anything that could pressure a user into risky action.

## Future Starknet connection

A future version of the Readiness Passport may support optional Starknet-based proof or attestation.

This should happen only after the user understands what is being shared.

A safe future flow could be:

```text
User completes readiness path
        ↓
User reviews what will be shared
        ↓
User chooses whether to create or link a proof
        ↓
Only limited proof data is shared
        ↓
Private learning history remains protected
```

## Design goal

The Readiness Passport should feel like a calm guide.

It should help users answer:

- What have I learned?
- What risks do I understand?
- What should I practice next?
- Am I ready to explore further?
- What can I safely share?

The passport exists to support better decisions, not to create pressure.
