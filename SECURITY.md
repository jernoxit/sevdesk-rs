# Security policy

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub's **Report a
vulnerability** button (private vulnerability reporting) on the "Security" tab
of this repository. Do not open a public issue for a security problem.

## Notes on tokens

- The crate never logs API tokens.
- A sevDesk API token has no scopes and no expiry and opens the whole
  bookkeeping of its account. If one leaks (in a commit, a log or an issue),
  rotate it in sevDesk at once; removing it from git history is not enough.
- Never run the live tests with a production token.
