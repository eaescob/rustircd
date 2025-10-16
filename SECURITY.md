# Security Policy

## Supported Versions

Currently supported versions with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in RustIRCd, please report it responsibly.

### How to Report

**DO NOT** open a public GitHub issue for security vulnerabilities.

Instead, please report security issues by emailing:
- **Email:** security@rustircd.org (or project maintainer)
- **Subject Line:** [SECURITY] Brief description

### What to Include

Please provide:
1. **Description** - Detailed description of the vulnerability
2. **Impact** - What an attacker could achieve
3. **Steps to Reproduce** - How to reproduce the issue
4. **Proof of Concept** - Code or steps demonstrating the issue
5. **Suggested Fix** - If you have ideas for remediation
6. **Your Contact Info** - For follow-up questions

### Response Timeline

- **Initial Response:** Within 48 hours
- **Status Update:** Within 7 days
- **Fix Timeline:** Depends on severity
  - Critical: 1-7 days
  - High: 7-14 days
  - Medium: 14-30 days
  - Low: Best effort

### Disclosure Policy

We follow **Coordinated Disclosure**:

1. You report the vulnerability privately
2. We acknowledge and investigate
3. We develop and test a fix
4. We release a security patch
5. We publish a security advisory
6. After 90 days (or sooner if fixed), details can be disclosed

We credit security researchers who report vulnerabilities (unless you prefer to remain anonymous).

## Security Audit

A comprehensive security audit was completed in October 2025. See:
- [SECURITY_AUDIT.md](docs/SECURITY_AUDIT.md) - Full audit report with 32 findings
- [SECURITY_SUMMARY.md](docs/SECURITY_SUMMARY.md) - Quick reference summary

### Known Issues

See the audit report for current security findings and remediation roadmap:
- **Critical:** 1 finding (password hashing)
- **High:** 4 findings (dependencies, authentication)
- **Medium:** 12 findings (DoS protection, authorization)
- **Low:** 8 findings (information disclosure, configuration)

A 12-week remediation roadmap is in progress.

## Security Features

RustIRCd includes several security features:

### Authentication & Authorization
- Operator authentication with password hashing (SHA-256, migrating to Argon2)
- Operator flag-based privilege system
- SASL authentication support (backend integration in progress)
- Hostmask validation for operators
- Services authentication (Atheme protocol)

### Network Security
- TLS/SSL support using rustls
- Server-to-server authentication
- Certificate validation (review in progress)
- DNS and ident lookup (RFC 1413 compliant)

### DoS Protection
- Connection throttling (multi-stage)
- Per-class sendq/recvq limits
- Buffer overflow protection
- Connection limits per IP/class
- Message rate limiting (in development)

### Input Validation
- Comprehensive IRC message parsing
- Nickname and channel name validation
- Parameter validation
- Configuration validation system

### Privacy & Information Security
- Configurable STATS information disclosure
- Operator-only access controls
- IP address hiding options
- Secret channel support

### Code Security
- **Zero `unsafe` blocks** - Full Rust memory safety
- No SQL injection (no SQL database)
- No command injection (proper parameter handling)
- Buffer overflow protection at language level

## Best Practices

### For Operators

1. **Passwords:**
   - Use strong, unique passwords for operators
   - Change default passwords immediately
   - Never share operator credentials
   - Rotate passwords regularly

2. **Configuration:**
   - Restrict config file permissions (`chmod 600`)
   - Use specific hostmasks, not wildcards (`*@*`)
   - Enable TLS for sensitive connections
   - Review allow blocks and class limits

3. **Monitoring:**
   - Monitor operator authentication attempts
   - Watch for unusual connection patterns
   - Review STATS regularly
   - Enable and review security logs

4. **Network:**
   - Use strong server link passwords
   - Enable TLS for server-to-server links
   - Restrict SQUIT privileges
   - Validate server certificates

### For Developers

1. **Code Review:**
   - Review all authentication/authorization code
   - Check for information disclosure in errors
   - Validate all user inputs
   - Use proper error handling (no `.unwrap()`)

2. **Dependencies:**
   - Run `cargo audit` regularly
   - Keep dependencies updated
   - Review dependency security advisories
   - Use minimal dependency set

3. **Testing:**
   - Write security-focused tests
   - Fuzz test parsers
   - Test privilege escalation scenarios
   - Test DoS resilience

4. **Deployment:**
   - Use read-only file systems where possible
   - Run with minimal privileges
   - Enable rate limiting
   - Configure firewalls properly

## Security Checklist

Before deploying RustIRCd in production:

### Configuration
- [ ] Config file has secure permissions (600)
- [ ] Strong operator passwords set
- [ ] Operator hostmasks are specific
- [ ] TLS configured with valid certificates
- [ ] Server link passwords are strong
- [ ] Throttling enabled
- [ ] Connection classes configured
- [ ] Allow blocks properly restricted

### Network
- [ ] Firewall rules configured
- [ ] DDoS protection in place
- [ ] Rate limiting configured
- [ ] Reverse proxy if needed (SSL termination)
- [ ] DNS properly configured

### Monitoring
- [ ] Log aggregation configured
- [ ] Security event alerts set up
- [ ] Operator action logging enabled
- [ ] Failed auth attempt monitoring
- [ ] Resource usage monitoring

### Updates
- [ ] Regular dependency audits scheduled
- [ ] Security patch process defined
- [ ] Backup and recovery tested
- [ ] Incident response plan documented

## Vulnerability History

### 2025

#### October 2025 - Security Audit Completed
- Comprehensive audit identified 32 findings
- No actively exploited vulnerabilities
- Remediation roadmap created
- See [SECURITY_AUDIT.md](docs/SECURITY_AUDIT.md) for details

## Resources

- **Full Security Audit:** [docs/SECURITY_AUDIT.md](docs/SECURITY_AUDIT.md)
- **Quick Reference:** [docs/SECURITY_SUMMARY.md](docs/SECURITY_SUMMARY.md)
- **Performance Guide:** [docs/PERFORMANCE.md](docs/PERFORMANCE.md)
- **Project Status:** [PROJECT_STATUS.md](PROJECT_STATUS.md)

### External Resources
- Rust Security Guidelines: https://anssi-fr.github.io/rust-guide/
- OWASP Secure Coding: https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/
- IRC Security FAQ: https://www.irchelp.org/security/
- RustSec Advisory DB: https://rustsec.org/

## Contact

For security issues: security@rustircd.org  
For general issues: GitHub Issues  
Project repository: https://github.com/emilio/rustircd

---

**Last Updated:** October 10, 2025  
**Security Audit:** October 10, 2025

