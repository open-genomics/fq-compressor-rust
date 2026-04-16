# Security Policy

## Supported Versions

We release patches for security vulnerabilities. Currently supported versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take the security of fqc seriously. If you believe you have found a security vulnerability, please report it to us as described below.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via GitHub's private vulnerability reporting feature:

1. Go to the [Security Advisories](https://github.com/LessUp/fq-compressor-rust/security/advisories) page
2. Click "Report a vulnerability"
3. Fill out the form with details about the vulnerability

Alternatively, you can email the maintainer directly at the address listed in the repository's CODEOWNERS file.

### What to Include

Please include the following information in your report:

- **Type of vulnerability** (e.g., buffer overflow, path traversal, etc.)
- **Affected component** (e.g., FASTQ parser, archive reader, etc.)
- **Steps to reproduce** the vulnerability
- **Proof-of-concept or exploit code** (if available)
- **Impact** of the vulnerability
- **Suggested fix** (if available)

### Response Timeline

We will respond to your report within 48 hours and will keep you informed of our progress:

1. **Acknowledgment**: Within 48 hours
2. **Initial Assessment**: Within 7 days
3. **Fix Development**: Depends on complexity (typically 7-14 days)
4. **CVE Assignment**: If applicable, we will request a CVE from GitHub
5. **Public Disclosure**: After a fix is released

### Disclosure Policy

We follow the principle of [Coordinated Vulnerability Disclosure](https://en.wikipedia.org/wiki/Coordinated_vulnerability_disclosure):

1. We will work with you to understand and fix the vulnerability
2. We will not disclose the vulnerability until a fix is available
3. We will credit you in the security advisory (unless you prefer to remain anonymous)
4. We will publish a security advisory on GitHub and update the CHANGELOG

### Safe Harbor

We consider security research conducted in good faith to be beneficial. We will not pursue legal action against researchers who:

- Make a good faith effort to avoid privacy violations and destruction of data
- Do not access, modify, or delete data that does not belong to them
- Report vulnerabilities in accordance with this policy
- Allow us a reasonable time to fix the issue before public disclosure

## Security Best Practices

When using fqc, we recommend the following security practices:

### Input Validation

- Only process FASTQ files from trusted sources
- Use `fqc verify` to check archive integrity before decompression
- Be cautious with files from untrusted sources (potential path traversal in filenames)

### Archive Handling

- Verify archives with `fqc verify` before relying on their contents
- Keep backups of original FASTQ files until you've verified the compression/decompression cycle

### File Permissions

- Ensure output files are written to directories with appropriate permissions
- Do not run fqc as root unless absolutely necessary

### Network Security

- When using fqc in pipelines, ensure intermediate files are protected
- Use secure methods (HTTPS, SSH) when transferring `.fqc` archives

## Known Security Considerations

### Memory Safety

fqc is written in Rust, which provides memory safety guarantees by default. However:

- We use `unsafe` code only in the Windows-specific memory detection module (`src/common/memory_budget.rs`)
- This code is marked with `#[allow(unsafe_code)]` and has been audited

### Denial of Service

Certain inputs could cause resource exhaustion:

- Malformed FASTQ files with extremely long lines
- Malformed `.fqc` archives with corrupt block headers

These are handled gracefully with error messages and appropriate exit codes.

### Path Traversal

Original filenames from FASTQ inputs are stored in the `.fqc` archive. When decompressing:

- Filenames are not used for output (output path is specified via `-o` flag)
- No risk of path traversal from stored filenames

## Security Updates

Security updates will be released as patch versions and announced via:

- GitHub Security Advisories
- CHANGELOG.md
- Release notes on GitHub

We recommend subscribing to GitHub's security alerts for this repository.

## Contact

For security-related questions that are not vulnerability reports, please open a GitHub Discussion or Issue with the `security` label.
