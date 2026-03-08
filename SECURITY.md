# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 2.x     | :white_check_mark: |
| < 2.0   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in Fastest, please report it responsibly:

1. **Do NOT open a public GitHub issue.**
2. Email **derens99@gmail.com** with:
   - A description of the vulnerability
   - Steps to reproduce
   - Potential impact
3. You will receive acknowledgement within 48 hours.
4. A fix will be developed and released as soon as possible.

## Security Measures

- All CI builds run `cargo audit` to detect known vulnerabilities in dependencies
- Dependabot is configured for automated dependency updates
- Generated Python code uses identifier validation to prevent injection
- Subprocess communication uses JSON serialization (no shell execution)
