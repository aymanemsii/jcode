# Network and Privacy Policy

This fork disables telemetry by default and by design.

## Telemetry

- No anonymous analytics are collected.
- No usage statistics are collected or sent.
- No crash reports are collected or sent.
- No first-run telemetry ping is sent.
- No telemetry notice/banner is shown in onboarding.
- The legacy telemetry APIs are retained as no-op compatibility shims where call sites still exist.

The client telemetry HTTP delivery path is inactive. Telemetry payload helpers return without creating an HTTP client, spawning a sender, or posting to a telemetry endpoint.

## Silent Background Requests

Silent background network requests are not part of this fork's design.

Automatic update checks are disabled by default. The update module still contains GitHub release, commit, checksum, and asset-download code for explicit update flows, but normal startup/session activity should not silently check for updates.

## Allowed External Network Behavior

External network requests should be limited to:

- Configured AI model/provider calls required for normal chat and agent execution.
- Provider authentication flows explicitly initiated by the user.
- Explicit user-enabled tools/actions that inherently require network access, such as web search, web fetch, code search, Gmail, Composio-backed actions, Telegram sending, browser/provider integrations, or explicit update commands.
- Localhost/TCP/WebSocket listeners and connections used for local app/server/gateway behavior.

## Network-Capable Areas Found During Audit

- `crates/jcode-provider-core`: shared provider HTTP clients used by configured model providers.
- `crates/jcode-app-core/src/update.rs`: GitHub release/commit checks and update asset/checksum downloads; automatic checks are disabled by default in this fork.
- `crates/jcode-app-core/src/tool/webfetch.rs`: explicit web fetch tool.
- `crates/jcode-app-core/src/tool/websearch.rs`: explicit web search tool.
- `crates/jcode-app-core/src/tool/codesearch.rs`: explicit code search tool.
- `crates/jcode-app-core/src/server/jade_relay.rs`: relay/client HTTP paths for configured relay behavior.
- `crates/jcode-app-core/src/channel.rs`: remote channel client paths for configured remote session/channel behavior.
- `crates/jcode-app-core/src/notifications.rs`: notification delivery client paths.
- `crates/jcode-base/src/auth/oauth.rs`: explicit OAuth token/profile requests and localhost callback listeners.
- `crates/jcode-base/src/auth/*`: explicit provider/device/OAuth flows, including Copilot, Cursor, Google/Antigravity, and live credential probes.
- `crates/jcode-base/src/gmail.rs`: explicit Gmail API operations.
- `crates/jcode-base/src/provider/*`: configured model-provider HTTP and WebSocket transports.
- `crates/jcode-base/src/sidecar.rs`: provider sidecar HTTP behavior.
- `crates/jcode-base/src/telegram.rs`: explicit Telegram message sending.
- `crates/jcode-base/src/gateway.rs` and server gateway code: local TCP/WebSocket gateway behavior.
- `crates/jcode-embedding/src/lib.rs`: network-capable embedding client path.
- `telemetry-worker/`: historical telemetry receiver infrastructure remains in the repository, but the client no longer posts telemetry to it.

When adding new network behavior, prefer explicit user action or configured provider/tool paths. Do not add analytics, telemetry, crash reporting, remote config, first-run pings, or silent external background requests.
