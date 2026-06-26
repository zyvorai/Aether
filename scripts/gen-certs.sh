#!/usr/bin/env bash
# ==============================================================================
# gen-certs.sh — Zeus OS / Aether certificate and key generator
# ==============================================================================
#
# Generates all cryptographic material needed to run the full stack:
#
#   1. Zyvor license signing keypair
#      keys/zyvor_private.pem   RSA-2048 PKCS#8 private key  (never commit)
#      keys/zyvor_public.pem    RSA-2048 PKCS#8 public key   (committed)
#
#   2. TLS server certificate (self-signed, for `aether serve --tls-cert`)
#      certs/server.key         RSA-2048 private key
#      certs/server.crt         X.509 self-signed certificate (SAN-enabled)
#      certs/server.pem         Combined cert + key (some tooling wants this)
#
#   3. TLS CA + server certificate (local CA, for mTLS / Kubernetes trust)
#      certs/ca.key             CA private key
#      certs/ca.crt             CA certificate
#      certs/server-ca.key      Server private key signed by local CA
#      certs/server-ca.crt      Server certificate signed by local CA
#
#   4. Mock IdP certificate (replaces packaging/mock-idp/cert.pem)
#      packaging/mock-idp/key.pem
#      packaging/mock-idp/cert.pem
#
# Usage:
#   ./scripts/gen-certs.sh [options]
#
# Options:
#   --all            Generate everything (default)
#   --license        Generate only the license signing keypair
#   --tls            Generate only the TLS server certificate
#   --ca             Generate only the CA + CA-signed server certificate
#   --mock-idp       Generate only the mock IdP certificate
#   --domain DOMAIN  SAN domain for TLS certs (default: localhost)
#   --days N         Validity period in days (default: 3650 = ~10 years)
#   --force          Overwrite existing files without prompting
#   --help           Show this message
#
# Requirements:
#   openssl (any modern version — LibreSSL or OpenSSL 1.1+)
#
# Exit codes:
#   0  Success
#   1  openssl not found or generation failed
# ==============================================================================

set -euo pipefail

# ── Defaults ──────────────────────────────────────────────────────────────────
DOMAIN="${DOMAIN:-localhost}"
DAYS="${DAYS:-3650}"
FORCE="${FORCE:-false}"
MODE="all"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KEYS_DIR="${REPO_ROOT}/keys"
CERTS_DIR="${REPO_ROOT}/certs"
MOCK_IDP_DIR="${REPO_ROOT}/packaging/mock-idp"

# ── Colors ────────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; BOLD='\033[1m'; RESET='\033[0m'
ok()   { printf "${GREEN}  ✓${RESET}  %s\n" "$*"; }
info() { printf "${BOLD}  →${RESET}  %s\n" "$*"; }
warn() { printf "${YELLOW}  ⚠${RESET}  %s\n" "$*"; }
die()  { printf "${RED}  ✗${RESET}  %s\n" "$*" >&2; exit 1; }
sep()  { printf "\n${BOLD}-- %s${RESET}\n" "$*"; }

# ── Argument parsing ──────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --all)       MODE="all"      ;;
    --license)   MODE="license"  ;;
    --tls)       MODE="tls"      ;;
    --ca)        MODE="ca"       ;;
    --mock-idp)  MODE="mock-idp" ;;
    --domain)    DOMAIN="$2"; shift ;;
    --days)      DAYS="$2";   shift ;;
    --force)     FORCE="true" ;;
    --help|-h)
      sed -n '/^# Usage/,/^# =\{10\}/p' "$0" | head -40
      exit 0 ;;
    *) die "Unknown option: $1. Use --help for usage." ;;
  esac
  shift
done

# ── Preflight ─────────────────────────────────────────────────────────────────
command -v openssl &>/dev/null || die "openssl is not installed. Install it first."
OPENSSL_VERSION="$(openssl version 2>&1 | head -1)"
info "Using: ${OPENSSL_VERSION}"
info "Domain: ${DOMAIN}  |  Validity: ${DAYS} days  |  Mode: ${MODE}"

# ── Helpers ───────────────────────────────────────────────────────────────────
# Guard: skip if file exists and --force is not set
guard() {
  local file="$1"
  if [[ -f "$file" && "$FORCE" != "true" ]]; then
    warn "$(basename "$file") already exists — skipping (use --force to overwrite)"
    return 1
  fi
  return 0
}

# Write an OpenSSL config with SANs to a temp file; caller must rm it.
make_san_config() {
  local out_file="$1"
  local cn="$2"
  cat > "$out_file" <<EOF
[req]
default_bits       = 2048
prompt             = no
default_md         = sha256
distinguished_name = dn
x509_extensions    = v3_req
req_extensions     = v3_req

[dn]
CN = ${cn}
O  = ZyvorAI Labs Private Limited
OU = Zeus OS
C  = IN

[v3_req]
subjectAltName = @alt_names
keyUsage       = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth, clientAuth
basicConstraints = CA:FALSE

[alt_names]
DNS.1 = ${DOMAIN}
DNS.2 = *.${DOMAIN}
DNS.3 = localhost
IP.1  = 127.0.0.1
IP.2  = ::1
EOF
}

make_ca_config() {
  local out_file="$1"
  cat > "$out_file" <<EOF
[req]
default_bits       = 4096
prompt             = no
default_md         = sha256
distinguished_name = dn
x509_extensions    = v3_ca

[dn]
CN = Zeus OS Local CA
O  = ZyvorAI Labs Private Limited
OU = Zeus OS Internal
C  = IN

[v3_ca]
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer
basicConstraints = critical, CA:TRUE, pathlen:1
keyUsage = critical, cRLSign, keyCertSign
EOF
}

# ── 1. License signing keypair ────────────────────────────────────────────────
gen_license_keypair() {
  sep "License signing keypair"
  mkdir -p "$KEYS_DIR"

  if guard "${KEYS_DIR}/zyvor_private.pem"; then
    info "Generating RSA-2048 private key → keys/zyvor_private.pem"
    openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
      -out "${KEYS_DIR}/zyvor_private.pem" 2>/dev/null
    chmod 600 "${KEYS_DIR}/zyvor_private.pem"
    ok "keys/zyvor_private.pem (mode 600)"
  fi

  if guard "${KEYS_DIR}/zyvor_public.pem"; then
    info "Extracting public key → keys/zyvor_public.pem"
    openssl pkey -in "${KEYS_DIR}/zyvor_private.pem" \
      -pubout -out "${KEYS_DIR}/zyvor_public.pem" 2>/dev/null
    ok "keys/zyvor_public.pem"
  fi

  # Verify the keypair
  PUB_FROM_PRIV="$(openssl pkey -in "${KEYS_DIR}/zyvor_private.pem" -pubout 2>/dev/null | sha256sum)"
  PUB_FROM_PUB="$(cat "${KEYS_DIR}/zyvor_public.pem" | sha256sum)"
  if [[ "$PUB_FROM_PRIV" == "$PUB_FROM_PUB" ]]; then
    ok "Keypair verified — public key matches private key"
  else
    die "Keypair mismatch — keys/zyvor_public.pem does not match keys/zyvor_private.pem"
  fi

  warn "NEVER commit keys/zyvor_private.pem — it is in .gitignore"
  info "Rebuild the aether binary after replacing keys/zyvor_public.pem"
  info "  so the new public key is embedded: cargo build --release"
}

# ── 2. Self-signed TLS server certificate ────────────────────────────────────
gen_tls_selfsigned() {
  sep "TLS server certificate (self-signed)"
  mkdir -p "$CERTS_DIR"

  local cfg; cfg="$(mktemp /tmp/zeus-tls-XXXXXX.cnf)"
  trap "rm -f '$cfg'" RETURN
  make_san_config "$cfg" "$DOMAIN"

  if guard "${CERTS_DIR}/server.key"; then
    info "Generating TLS private key → certs/server.key"
    openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
      -out "${CERTS_DIR}/server.key" 2>/dev/null
    chmod 600 "${CERTS_DIR}/server.key"
    ok "certs/server.key (mode 600)"
  fi

  if guard "${CERTS_DIR}/server.crt"; then
    info "Generating self-signed certificate → certs/server.crt"
    openssl req -new -x509 -sha256 \
      -key "${CERTS_DIR}/server.key" \
      -out "${CERTS_DIR}/server.crt" \
      -days "$DAYS" \
      -config "$cfg" \
      -extensions v3_req 2>/dev/null
    ok "certs/server.crt (valid ${DAYS} days, SAN: ${DOMAIN}, localhost, 127.0.0.1)"
  fi

  if guard "${CERTS_DIR}/server.pem"; then
    cat "${CERTS_DIR}/server.crt" "${CERTS_DIR}/server.key" > "${CERTS_DIR}/server.pem"
    chmod 600 "${CERTS_DIR}/server.pem"
    ok "certs/server.pem (combined cert+key)"
  fi

  # Print fingerprint
  info "Certificate fingerprint (SHA-256):"
  openssl x509 -in "${CERTS_DIR}/server.crt" -fingerprint -sha256 -noout 2>/dev/null \
    | sed 's/^/     /'

  printf "\n  Start the server with TLS:\n"
  printf "  ${BOLD}aether serve --tls-cert certs/server.crt --tls-key certs/server.key${RESET}\n\n"
  printf "  Trust the cert on macOS:\n"
  printf "  ${BOLD}sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain certs/server.crt${RESET}\n\n"
  printf "  Trust the cert on Linux:\n"
  printf "  ${BOLD}sudo cp certs/server.crt /usr/local/share/ca-certificates/zeus-os.crt && sudo update-ca-certificates${RESET}\n"
}

# ── 3. CA + CA-signed server certificate ─────────────────────────────────────
gen_ca_signed() {
  sep "Local CA + CA-signed server certificate"
  mkdir -p "$CERTS_DIR"

  local ca_cfg; ca_cfg="$(mktemp /tmp/zeus-ca-XXXXXX.cnf)"
  local srv_cfg; srv_cfg="$(mktemp /tmp/zeus-srv-XXXXXX.cnf)"
  trap "rm -f '$ca_cfg' '$srv_cfg'" RETURN
  make_ca_config "$ca_cfg"
  make_san_config "$srv_cfg" "$DOMAIN"

  # CA key
  if guard "${CERTS_DIR}/ca.key"; then
    info "Generating CA private key → certs/ca.key"
    openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:4096 \
      -out "${CERTS_DIR}/ca.key" 2>/dev/null
    chmod 600 "${CERTS_DIR}/ca.key"
    ok "certs/ca.key (4096-bit, mode 600)"
  fi

  # CA certificate
  if guard "${CERTS_DIR}/ca.crt"; then
    info "Generating CA certificate → certs/ca.crt"
    openssl req -new -x509 -sha256 \
      -key "${CERTS_DIR}/ca.key" \
      -out "${CERTS_DIR}/ca.crt" \
      -days "$DAYS" \
      -config "$ca_cfg" \
      -extensions v3_ca 2>/dev/null
    ok "certs/ca.crt (valid ${DAYS} days)"
  fi

  # Server key
  if guard "${CERTS_DIR}/server-ca.key"; then
    info "Generating server private key → certs/server-ca.key"
    openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
      -out "${CERTS_DIR}/server-ca.key" 2>/dev/null
    chmod 600 "${CERTS_DIR}/server-ca.key"
    ok "certs/server-ca.key (mode 600)"
  fi

  # Server CSR + sign with CA
  local csr; csr="$(mktemp /tmp/zeus-srv-XXXXXX.csr)"
  trap "rm -f '$csr'" RETURN

  info "Generating CSR and signing with CA → certs/server-ca.crt"
  openssl req -new -sha256 \
    -key "${CERTS_DIR}/server-ca.key" \
    -out "$csr" \
    -config "$srv_cfg" 2>/dev/null

  if guard "${CERTS_DIR}/server-ca.crt"; then
    openssl x509 -req -sha256 \
      -in "$csr" \
      -CA "${CERTS_DIR}/ca.crt" \
      -CAkey "${CERTS_DIR}/ca.key" \
      -CAcreateserial \
      -out "${CERTS_DIR}/server-ca.crt" \
      -days "$DAYS" \
      -extfile "$srv_cfg" \
      -extensions v3_req 2>/dev/null
    ok "certs/server-ca.crt (signed by Zeus OS Local CA)"
  fi

  # Verify chain
  if openssl verify -CAfile "${CERTS_DIR}/ca.crt" "${CERTS_DIR}/server-ca.crt" &>/dev/null; then
    ok "Certificate chain verified"
  else
    die "Certificate chain verification failed"
  fi

  info "Certificate fingerprint (SHA-256):"
  openssl x509 -in "${CERTS_DIR}/server-ca.crt" -fingerprint -sha256 -noout 2>/dev/null \
    | sed 's/^/     /'

  printf "\n  Start the server with CA-signed TLS:\n"
  printf "  ${BOLD}aether serve --tls-cert certs/server-ca.crt --tls-key certs/server-ca.key${RESET}\n\n"
  printf "  Trust the CA on macOS:\n"
  printf "  ${BOLD}sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain certs/ca.crt${RESET}\n\n"
  printf "  Trust the CA on Linux:\n"
  printf "  ${BOLD}sudo cp certs/ca.crt /usr/local/share/ca-certificates/zeus-os-ca.crt && sudo update-ca-certificates${RESET}\n"
}

# ── 4. Mock IdP certificate (for dev SSO testing) ─────────────────────────────
gen_mock_idp() {
  sep "Mock IdP certificate (packaging/mock-idp)"
  mkdir -p "$MOCK_IDP_DIR"

  local cfg; cfg="$(mktemp /tmp/zeus-idp-XXXXXX.cnf)"
  trap "rm -f '$cfg'" RETURN

  cat > "$cfg" <<EOF
[req]
default_bits       = 2048
prompt             = no
default_md         = sha256
distinguished_name = dn
x509_extensions    = v3_ext

[dn]
CN = Aether Mock IdP
O  = ZyvorAI Labs Private Limited
C  = IN

[v3_ext]
subjectKeyIdentifier   = hash
authorityKeyIdentifier = keyid,issuer
basicConstraints       = CA:FALSE
keyUsage               = critical, digitalSignature, nonRepudiation
extendedKeyUsage       = serverAuth, emailProtection
subjectAltName         = @sans

[sans]
DNS.1 = localhost
DNS.2 = aether-mock-idp
IP.1  = 127.0.0.1
EOF

  if guard "${MOCK_IDP_DIR}/key.pem"; then
    info "Generating mock IdP private key → packaging/mock-idp/key.pem"
    openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
      -out "${MOCK_IDP_DIR}/key.pem" 2>/dev/null
    chmod 600 "${MOCK_IDP_DIR}/key.pem"
    ok "packaging/mock-idp/key.pem (mode 600)"
  fi

  if guard "${MOCK_IDP_DIR}/cert.pem"; then
    info "Generating mock IdP self-signed certificate → packaging/mock-idp/cert.pem"
    openssl req -new -x509 -sha256 \
      -key "${MOCK_IDP_DIR}/key.pem" \
      -out "${MOCK_IDP_DIR}/cert.pem" \
      -days "$DAYS" \
      -config "$cfg" \
      -extensions v3_ext 2>/dev/null
    ok "packaging/mock-idp/cert.pem (valid ${DAYS} days)"
  fi

  # Verify self-signature
  if openssl verify -CAfile "${MOCK_IDP_DIR}/cert.pem" "${MOCK_IDP_DIR}/cert.pem" &>/dev/null; then
    ok "Mock IdP certificate is self-consistent"
  fi

  warn "Rebuild the binary after rotating the mock IdP cert:"
  info "  cargo build   (embeds packaging/mock-idp/cert.pem via include_str!)"
}

# ── Summary ───────────────────────────────────────────────────────────────────
print_summary() {
  sep "Summary"
  printf "\n  Files generated under ${BOLD}${REPO_ROOT}${RESET}:\n\n"

  local files=(
    "keys/zyvor_private.pem  ← license signing private key  [KEEP SECRET]"
    "keys/zyvor_public.pem   ← license signing public key   [commit this]"
    "certs/ca.key            ← local CA private key         [keep secret]"
    "certs/ca.crt            ← local CA certificate         [trust this]"
    "certs/server.key        ← self-signed TLS private key"
    "certs/server.crt        ← self-signed TLS certificate"
    "certs/server.pem        ← combined cert + key (PEM)"
    "certs/server-ca.key     ← CA-signed TLS private key"
    "certs/server-ca.crt     ← CA-signed TLS certificate"
    "packaging/mock-idp/key.pem   ← mock IdP private key"
    "packaging/mock-idp/cert.pem  ← mock IdP certificate"
  )

  for f in "${files[@]}"; do
    local path="${f%% *}"
    if [[ -f "${REPO_ROOT}/${path}" ]]; then
      printf "  ${GREEN}✓${RESET}  %s\n" "$f"
    else
      printf "  ${YELLOW}–${RESET}  %s  (not generated this run)\n" "$f"
    fi
  done
  echo
}

# ── Main ──────────────────────────────────────────────────────────────────────
echo
printf "${BOLD}Zeus OS / Aether — Certificate Generator${RESET}\n"
echo

case "$MODE" in
  all)
    gen_license_keypair
    gen_tls_selfsigned
    gen_ca_signed
    gen_mock_idp
    ;;
  license)  gen_license_keypair ;;
  tls)      gen_tls_selfsigned  ;;
  ca)       gen_ca_signed       ;;
  mock-idp) gen_mock_idp        ;;
esac

print_summary
