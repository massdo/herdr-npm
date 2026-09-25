# Security policy

## Supported code

Releases are published as `v<version>` tags on
[GitHub](https://github.com/massdo/herdr-npm/releases). Security fixes land on
`main` and ship in the next release; older releases and commits do not receive
backports. Use the latest release.

## Reporting a vulnerability

Use GitHub's **Report a vulnerability** form at
<https://github.com/massdo/herdr-npm/security/advisories/new>. Include the
affected commit, operating system, Herdr version, impact, and a minimal
reproduction without credentials or personal data.

If the form is unavailable, do not post vulnerability details in a public issue
or pull request; wait until a private reporting channel is available.

## Trust boundary

herdr-npm reads the nearest `package.json`. Inside a declared workspace, it also
reads the workspace declaration (`pnpm-workspace.yaml` or the `workspaces` field
of the root `package.json`) and the `package.json` of each member. It launches
the selected npm or pnpm script in a Herdr shell tab, in the directory of that
script's package. It does not sandbox scripts or their lifecycle
hooks: they run with the user's permissions and the shell's environment and
can access files, credentials, and the network. Review a project's scripts
before running them. Quoting script names prevents shell argument injection;
it does not make the script body safe.

## Prebuilt binaries

Releases publish binaries for macOS arm64, macOS x86_64 and Linux x86_64
(statically linked with musl), with `SHA256SUMS` and `SOURCE_COMMIT`. At
install time, `scripts/fetch-or-build.sh` keeps a binary only when
`SOURCE_COMMIT` is the installed commit, no tracked file of the checkout is
modified, and the file matches its single `SHA256SUMS` entry; anything else
builds from source. These files detect corruption and a commit mismatch.
They come from the same GitHub release as the binary, so they do not
authenticate it independently: trust rests on HTTPS to GitHub and on this
repository's release workflow. Releases are published as GitHub immutable
releases, so their assets and tag cannot change afterwards. Binaries are not
signed or notarized.
