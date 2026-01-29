# Security Policy

## Supported Versions

Only the latest version of FeatherGate receives security updates. Users are encouraged to upgrade to the newest version.

## Reporting a Vulnerability

If you discover a security vulnerability in FeatherGate, please report it responsibly.

### How to Report

1. **Do NOT open a public issue** - this alerts malicious actors
2. Send an email to: `security@example.com` (replace with actual security email)
3. Include as much information as possible:
   - Version of FeatherGate
   - Environment details (OS, Rust version)
   - Steps to reproduce
   - Potential impact
   - Any proof-of-concept code

### Response Timeline

- **Within 48 hours**: Initial acknowledgment of receipt
- **Within 7 days**: Detailed assessment and timeline
- **Within 30 days**: Fix release for critical vulnerabilities

### Security Best Practices

#### For Users
- Use environment variables for API keys, never hardcode them
- Run FeatherGate behind reverse proxy with SSL/TLS
- Restrict API access with firewalls
- Monitor logs for unusual activity
- Keep dependencies updated

#### For Deployments
- Enable HTTPS in production
- Use API key rotation policies
- Implement rate limiting at proxy level
- Set up monitoring and alerting
- Regular security updates

## Security Features

### Built-in Protections
- Request size limits to prevent DoS attacks
- Error response truncation to prevent information leakage
- Input validation for all configuration parameters
- Safe HTTP client configuration with timeouts

### API Security
- No authentication required for basic usage (simplifies deployment)
- Compatible with external authentication solutions
- Supports API key rotation through configuration reload
- Request/response logging for security monitoring

## Vulnerability Disclosure

We follow responsible disclosure principles:

1. We investigate all reported vulnerabilities
2. We work with reporters to understand the issue
3. We provide a timeline for fixes
4. We coordinate public disclosure with the reporter
5. We credit reporters for their contributions

## Security Dependencies

FeatherGate uses these security-focused dependencies:
- `hyper` - HTTP server with security-first design
- `tokio` - Async runtime with built-in protection
- `serde` - Safe serialization with validation
- `thiserror` - Type-safe error handling

## Additional Resources

- [Rust Security Guidelines](https://rust-lang.org/guidelines/security.html)
- [Common Vulnerabilities and Exposures](https://cve.mitre.org/)
- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)