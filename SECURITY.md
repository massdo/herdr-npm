# Security policy

## Supported code

Security fixes target the latest code on `main`. No stable release has been
published yet, and older commits do not receive separate security backports.

## Reporting a vulnerability

Use GitHub's **Report a vulnerability** form at
<https://github.com/massdo/herdr-npm/security/advisories/new>. Include the
affected commit, operating system, Herdr version, impact, and a minimal
reproduction without credentials or personal data.

If the form is unavailable, do not post vulnerability details in a public issue
or pull request; wait until a private reporting channel is available.

## Trust boundary

herdr-npm reads the nearest `package.json` and launches the selected npm or pnpm
script in a Herdr shell tab. It does not sandbox scripts or their lifecycle
hooks: they run with the user's permissions and the shell's environment and
can access files, credentials, and the network. Review a project's scripts
before running them. Quoting script names prevents shell argument injection;
it does not make the script body safe.
