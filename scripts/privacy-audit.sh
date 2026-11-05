#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Known crates that provide network clients or transports. Keep URL parsing and
# TLS primitives out of this list: those alone cannot initiate network I/O.
network_crates='^(reqwest|hyper|ureq|isahc|surf|awc|curl|libcurl|tonic|grpcio|quinn|tungstenite|tokio-tungstenite|async-tungstenite|websocket|ws|hickory-resolver|trust-dns-resolver)(-|$)'

echo 'Checking normal dependency tree for network clients...'
if ! command -v cargo >/dev/null 2>&1; then
  echo 'error: cargo is required for the privacy dependency audit' >&2
  exit 2
fi

dependencies="$(cargo tree --edges normal --prefix none | sed -E 's/ v[^ ]+.*$//' | sort -u)"
network_dependencies="$(printf '%s\n' "$dependencies" | grep -E "$network_crates" || true)"
if [[ -n "$network_dependencies" ]]; then
  echo 'error: network-capable dependency found in the normal dependency tree:' >&2
  printf '  %s\n' "$network_dependencies" >&2
  exit 1
fi

echo 'Scanning Rust application source for direct network APIs...'
network_source='(std|tokio|async_std)[[:space:]]*::[[:space:]]*net[[:space:]]*::|Tcp(Stream|Listener)|Udp(Socket)|reqwest[[:space:]]*::|hyper[[:space:]]*::|ureq[[:space:]]*::|isahc[[:space:]]*::|surf[[:space:]]*::|tonic[[:space:]]*::|quinn[[:space:]]*::|tungstenite[[:space:]]*::|std[[:space:]]*::[[:space:]]*net[[:space:]]*::'
if grep -RInE --include='*.rs' "$network_source" src; then
  echo 'error: direct network API reference found under src/' >&2
  exit 1
fi

echo 'Privacy audit passed: no known network client dependency or direct network API found.'
