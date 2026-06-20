# PASS 25E-C — Admin invite onboarding backend

This pass adds the backend onboarding route for delegated Spark admins and moderators.

## Route namespace

```txt
/api/admin/onboarding
```

## Flow

```txt
POST /invite/inspect
POST /invite/email/request
POST /invite/email/confirm
POST /invite/password
POST /invite/totp/setup
POST /invite/totp/confirm
POST /invite/accept
```

A delegated admin role is activated only after:

1. The raw invite token matches `admin_invitations.token_hash`.
2. The submitted email matches the invited email.
3. The email OTP is confirmed.
4. A short-lived email proof token is issued.
5. Password is set for the invited account.
6. Authenticator TOTP is configured and confirmed.
7. Invite acceptance verifies password + TOTP again.
8. The active `admin_role_assignments` row is created from the invitation role/capabilities.

## Security notes

- Raw invite tokens are never stored.
- OTP values are hashed and single-use.
- Email proof tokens are short-lived and stored only as hashes in OTP metadata.
- The onboarding route does not accept arbitrary roles/capabilities from the client.
- `admin`/`moderator` activation uses the existing `admin_role_assignments` table only at final acceptance.

## Temporary bootstrap flag

Manual token/OTP return still depends on:

```env
SPARK_ADMIN_INVITE_RETURN_BOOTSTRAP_TOKENS=true
```

This is for early manual testing only. Disable it when real email delivery is active.

## Audit

```bash
node scripts/audit-pass25e-c-admin-invite-onboarding.mjs
```

Expected:

```txt
PASS 25E-C admin invite onboarding audit
OK: invite-token onboarding now validates invite token, email OTP proof, password, TOTP, and activates delegated admin roles only after acceptance.
```
