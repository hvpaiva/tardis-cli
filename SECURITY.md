# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in this project, please report it responsibly.

**Do not open a public issue.**

You can report vulnerabilities through either channel:

1. **GitHub Private Reporting** (preferred): use the [Report a vulnerability](https://github.com/hvpaiva/tardis-cli/security/advisories/new) button on the Security tab.
2. **Email**: send details to [contact@hvpaiva.dev](mailto:contact@hvpaiva.dev).

Please include:

- A description of the vulnerability
- Steps to reproduce
- Potential impact

You should receive a response within 48 hours. We will work with you to understand and address the issue before any public disclosure.

## Security Measures

This project uses the following automated security tools:

- **Dependabot alerts** for vulnerable dependencies
- **Secret scanning** to prevent accidental credential leaks
- **CodeQL analysis** for static code vulnerability detection (Rust)
- **cargo-deny** in CI to audit dependencies on every push

## Supported Versions

| Version | Supported |
|---------|-----------|
| latest  | Yes       |
