# Security Policy

## Supported versions

| Version | Status |
| --- | --- |
| 0.1.x | Supported |

## Reporting a vulnerability

Please **do not** report security issues in a public issue.

Use GitHub's private vulnerability reporting flow instead:

1. Open the repository's **Security** tab.
2. Go to **Advisories**.
3. Choose **Report a vulnerability**.

Include the affected command or component, a reproduction path, impact, and any mitigation notes you already have.

## Notes for users

- Verify archives from untrusted sources with `fqc verify` before depending on them.
- Keep original FASTQ data until you have validated a compression/decompression cycle.
- Prefer explicit output paths rather than relying on stored source filename metadata.
