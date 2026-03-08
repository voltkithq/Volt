# Contract Compatibility Annotations

When a contract fixture under `contracts/*.json` changes, add or update an annotation file in this directory.

Filename recommendation:

- `YYYY-MM-DD-<short-description>.md`

Required fields:

```md
# Contract Compatibility Annotation

- Contract: contracts/native-event-payloads.json
- Change-Type: backward-compatible
- Changelog-Updated: yes
- Notes: Added optional field `foo` to `ipc-message`.
```

Allowed `Change-Type` values:

- `backward-compatible`
- `breaking`
- `internal`

CI gate:

- `scripts/contracts/check-compatibility.mjs`
- Enforced in `.github/workflows/ci.yml`

Rules:

- Contract fixture changes must include:
  - annotation update in `contracts/annotations/`
  - `CHANGELOG.md` update
- If `Change-Type: breaking`, update `docs/compatibility-policy.md` in the same change.

