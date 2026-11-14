# Security policy

Milano Pen is a local-only desktop app: it runs no server, makes no network
requests of its own and stores everything on your Mac. Security issues still
matter — for example a crafted board export, image or link that could run code,
read files outside the app's data folder or corrupt data.

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

Please **do not open a public issue**. Use GitHub's private reporting instead:
**Security → Report a vulnerability** on the repository page. Include steps to
reproduce, the affected version and macOS version, and the impact you expect.

We aim to acknowledge reports within a week and to ship a fix or mitigation
as soon as practical. We will credit you in the release notes unless you
prefer otherwise.

## Security design (summary)

- The webview has no filesystem, shell or dialog permissions (see
  `src-tauri/capabilities/default.json`). Files are imported, exported and
  opened only through Rust commands, after a native dialog; pasted and dropped
  files reach Rust as bytes, never as paths.
- Imported files are served to the webview by a read-only `milano://`
  protocol that resolves only content hashes inside the asset folders.
- A strict Content-Security-Policy (`src-tauri/tauri.conf.json`) limits
  scripts to the app bundle and frames to YouTube (nocookie), Vimeo and Spotify.
- Rich text is rendered from its schema; stored HTML is never injected. Links
  are limited to `http`, `https` and `mailto`.
- Release builds are produced only by GitHub Actions from tagged commits, with
  SHA-256 checksums. Builds are signed and notarized only when the maintainers
  have configured Apple credentials; release notes always say which.
