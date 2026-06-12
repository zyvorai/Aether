#!/usr/bin/env bash
# Shared Kubernetes manifest fragments for deploy-k8s.sh and deploy-remote.sh.
# shellcheck shell=bash

# imagePullPolicy: Never for localhost/* ; IfNotPresent otherwise.
aether_deploy_image_pull_policy() {
  local image="${1:-}"
  case "${image}" in
    localhost/* | 127.0.0.1/*) echo "Never" ;;
    *) echo "${AETHER_IMAGE_PULL_POLICY:-IfNotPresent}" ;;
  esac
}

# Emit optional Secret + env YAML blocks for HA / auth (reads caller env).
# Sets global-ish vars: AETHER_MANIFEST_SECRETS_YAML, AETHER_MANIFEST_EXTRA_ENV_YAML
aether_deploy_build_secret_env_blocks() {
  local ns="${1:?namespace}"
  AETHER_MANIFEST_SECRETS_YAML=""
  AETHER_MANIFEST_EXTRA_ENV_YAML=""

  if [ -n "${AETHER_API_KEY:-}" ]; then
    AETHER_MANIFEST_SECRETS_YAML+="$(cat <<EOF

---
apiVersion: v1
kind: Secret
metadata:
  name: aether-api-key
  namespace: ${ns}
type: Opaque
stringData:
  api-key: ${AETHER_API_KEY}
EOF
)"
    AETHER_MANIFEST_EXTRA_ENV_YAML+="$(cat <<'EOF'

        - name: AETHER_API_KEY
          valueFrom:
            secretKeyRef:
              name: aether-api-key
              key: api-key
EOF
)"
  fi

  if [ -n "${AETHER_STATE_DATABASE_URL:-}" ]; then
    AETHER_MANIFEST_SECRETS_YAML+="$(cat <<EOF

---
apiVersion: v1
kind: Secret
metadata:
  name: aether-state-db
  namespace: ${ns}
type: Opaque
stringData:
  database-url: ${AETHER_STATE_DATABASE_URL}
EOF
)"
    AETHER_MANIFEST_EXTRA_ENV_YAML+="$(cat <<'EOF'

        - name: AETHER_STATE_DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: aether-state-db
              key: database-url
EOF
)"
    if [ -n "${AETHER_STATE_POLL_SECS:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_STATE_POLL_SECS
          value: \"${AETHER_STATE_POLL_SECS}\""
    fi
  fi

  if [ -n "${AETHER_REDIS_URL:-}" ]; then
    AETHER_MANIFEST_SECRETS_YAML+="$(cat <<EOF

---
apiVersion: v1
kind: Secret
metadata:
  name: aether-redis
  namespace: ${ns}
type: Opaque
stringData:
  redis-url: ${AETHER_REDIS_URL}
EOF
)"
    AETHER_MANIFEST_EXTRA_ENV_YAML+="$(cat <<'EOF'

        - name: AETHER_REDIS_URL
          valueFrom:
            secretKeyRef:
              name: aether-redis
              key: redis-url
EOF
)"
  fi

  if [ -n "${AETHER_OIDC_ISSUER:-}" ]; then
    AETHER_MANIFEST_SECRETS_YAML+="$(cat <<EOF

---
apiVersion: v1
kind: Secret
metadata:
  name: aether-oidc
  namespace: ${ns}
type: Opaque
stringData:
  issuer: ${AETHER_OIDC_ISSUER}
  client-id: ${AETHER_OIDC_CLIENT_ID:-}
  redirect-uri: ${AETHER_OIDC_REDIRECT_URI:-}
  session-secret: ${AETHER_SESSION_SECRET:-}
  client-secret: ${AETHER_OIDC_CLIENT_SECRET:-}
  role-map: ${AETHER_OIDC_ROLE_MAP:-}
EOF
)"
    AETHER_MANIFEST_EXTRA_ENV_YAML+="$(cat <<'EOF'

        - name: AETHER_OIDC_ISSUER
          valueFrom:
            secretKeyRef:
              name: aether-oidc
              key: issuer
        - name: AETHER_OIDC_CLIENT_ID
          valueFrom:
            secretKeyRef:
              name: aether-oidc
              key: client-id
        - name: AETHER_OIDC_REDIRECT_URI
          valueFrom:
            secretKeyRef:
              name: aether-oidc
              key: redirect-uri
        - name: AETHER_SESSION_SECRET
          valueFrom:
            secretKeyRef:
              name: aether-oidc
              key: session-secret
EOF
)"
    if [ -n "${AETHER_OIDC_CLIENT_SECRET:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_OIDC_CLIENT_SECRET
          valueFrom:
            secretKeyRef:
              name: aether-oidc
              key: client-secret"
    fi
    if [ -n "${AETHER_OIDC_ROLE_MAP:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_OIDC_ROLE_MAP
          valueFrom:
            secretKeyRef:
              name: aether-oidc
              key: role-map"
    fi
  fi

  if [ -n "${AETHER_LDAP_URL:-}" ]; then
    AETHER_MANIFEST_SECRETS_YAML+="$(cat <<EOF

---
apiVersion: v1
kind: Secret
metadata:
  name: aether-ldap
  namespace: ${ns}
type: Opaque
stringData:
  url: ${AETHER_LDAP_URL}
  base-dn: ${AETHER_LDAP_BASE_DN:-}
  domain: ${AETHER_LDAP_DOMAIN:-}
  session-secret: ${AETHER_SESSION_SECRET:-}
  bind-dn: ${AETHER_LDAP_BIND_DN:-}
  bind-password: ${AETHER_LDAP_BIND_PASSWORD:-}
  role-map: ${AETHER_LDAP_ROLE_MAP:-}
EOF
)"
    AETHER_MANIFEST_EXTRA_ENV_YAML+="$(cat <<'EOF'

        - name: AETHER_LDAP_URL
          valueFrom:
            secretKeyRef:
              name: aether-ldap
              key: url
        - name: AETHER_LDAP_BASE_DN
          valueFrom:
            secretKeyRef:
              name: aether-ldap
              key: base-dn
        - name: AETHER_SESSION_SECRET
          valueFrom:
            secretKeyRef:
              name: aether-ldap
              key: session-secret
EOF
)"
    if [ -n "${AETHER_LDAP_DOMAIN:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_LDAP_DOMAIN
          valueFrom:
            secretKeyRef:
              name: aether-ldap
              key: domain"
    fi
    if [ -n "${AETHER_LDAP_BIND_DN:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_LDAP_BIND_DN
          valueFrom:
            secretKeyRef:
              name: aether-ldap
              key: bind-dn
        - name: AETHER_LDAP_BIND_PASSWORD
          valueFrom:
            secretKeyRef:
              name: aether-ldap
              key: bind-password"
    fi
    if [ -n "${AETHER_LDAP_ROLE_MAP:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_LDAP_ROLE_MAP
          valueFrom:
            secretKeyRef:
              name: aether-ldap
              key: role-map"
    fi
    if [ -n "${AETHER_LDAP_USER_FILTER:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_LDAP_USER_FILTER
          value: \"${AETHER_LDAP_USER_FILTER}\""
    fi
    if [ -n "${AETHER_LDAP_DEFAULT_ROLE:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_LDAP_DEFAULT_ROLE
          value: \"${AETHER_LDAP_DEFAULT_ROLE}\""
    fi
  fi

  if [ -n "${AETHER_BACKUP_REMOTE_URL:-}" ]; then
    AETHER_MANIFEST_SECRETS_YAML+="$(cat <<EOF

---
apiVersion: v1
kind: Secret
metadata:
  name: aether-backup-remote
  namespace: ${ns}
type: Opaque
stringData:
  url: ${AETHER_BACKUP_REMOTE_URL}
  token: ${AETHER_BACKUP_REMOTE_TOKEN:-}
EOF
)"
    AETHER_MANIFEST_EXTRA_ENV_YAML+="$(cat <<'EOF'

        - name: AETHER_BACKUP_REMOTE_URL
          valueFrom:
            secretKeyRef:
              name: aether-backup-remote
              key: url
EOF
)"
    if [ -n "${AETHER_BACKUP_REMOTE_TOKEN:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_BACKUP_REMOTE_TOKEN
          valueFrom:
            secretKeyRef:
              name: aether-backup-remote
              key: token"
    fi
  fi

  if [ -n "${AETHER_OPA_URL:-}" ]; then
    AETHER_MANIFEST_SECRETS_YAML+="$(cat <<EOF

---
apiVersion: v1
kind: Secret
metadata:
  name: aether-opa
  namespace: ${ns}
type: Opaque
stringData:
  url: ${AETHER_OPA_URL}
EOF
)"
    AETHER_MANIFEST_EXTRA_ENV_YAML+="$(cat <<'EOF'

        - name: AETHER_OPA_URL
          valueFrom:
            secretKeyRef:
              name: aether-opa
              key: url
EOF
)"
    if [ -n "${AETHER_OPA_PACKAGE:-}" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_OPA_PACKAGE
          value: \"${AETHER_OPA_PACKAGE}\""
    fi
    if [ "${AETHER_OPA_ENFORCE:-}" = "1" ] || [ "${AETHER_OPA_ENFORCE:-}" = "true" ]; then
      AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_OPA_ENFORCE
          value: \"1\""
    fi
  fi

  if [ -n "${AETHER_AUDIT_WEBHOOK_URL:-}" ]; then
    AETHER_MANIFEST_SECRETS_YAML+="$(cat <<EOF

---
apiVersion: v1
kind: Secret
metadata:
  name: aether-audit-webhook
  namespace: ${ns}
type: Opaque
stringData:
  url: ${AETHER_AUDIT_WEBHOOK_URL}
EOF
)"
    AETHER_MANIFEST_EXTRA_ENV_YAML+="$(cat <<'EOF'

        - name: AETHER_AUDIT_WEBHOOK_URL
          valueFrom:
            secretKeyRef:
              name: aether-audit-webhook
              key: url
EOF
)"
  fi

  if [ -n "${AETHER_PACKETWOLF_URL:-}" ]; then
    AETHER_MANIFEST_EXTRA_ENV_YAML+="
        - name: AETHER_PACKETWOLF_URL
          value: \"${AETHER_PACKETWOLF_URL}\""
    if [ -n "${AETHER_PACKETWOLF_API_KEY:-}" ]; then
      AETHER_MANIFEST_SECRETS_YAML+="$(cat <<EOF

---
apiVersion: v1
kind: Secret
metadata:
  name: aether-packetwolf
  namespace: ${ns}
type: Opaque
stringData:
  api_key: ${AETHER_PACKETWOLF_API_KEY}
EOF
)"
      AETHER_MANIFEST_EXTRA_ENV_YAML+="$(cat <<'EOF'

        - name: AETHER_PACKETWOLF_API_KEY
          valueFrom:
            secretKeyRef:
              name: aether-packetwolf
              key: api_key
EOF
)"
    fi
  fi
}

# Service + optional Ingress. EXPOSE: nodeport | ingress | both (default nodeport).
aether_deploy_service_ingress_yaml() {
  local ns="${1:?}"
  local expose="${2:-${AETHER_EXPOSE:-nodeport}}"
  local node_port="${3:-${AETHER_NODE_PORT:-30090}}"
  local ingress_host="${4:-${AETHER_INGRESS_HOST:-}}"
  local ingress_class="${5:-${AETHER_INGRESS_CLASS:-}}"
  local tls_secret="${6:-${AETHER_INGRESS_TLS_SECRET:-}}"

  local out=""
  local svc_type="ClusterIP"
  case "${expose}" in
    nodeport|both) svc_type="NodePort" ;;
  esac

  out="$(cat <<EOF
---
apiVersion: v1
kind: Service
metadata:
  name: aether
  namespace: ${ns}
spec:
  type: ${svc_type}
  selector:
    app: aether
  ports:
  - name: http
    port: 5090
    targetPort: 5090
    protocol: TCP
EOF
)"
  if [ "${svc_type}" = "NodePort" ]; then
    out+="
    nodePort: ${node_port}"
  fi

  if { [ "${expose}" = "ingress" ] || [ "${expose}" = "both" ]; } && [ -n "${ingress_host}" ]; then
    local tls_block=""
    local annot_yaml=""
    if [ -n "${tls_secret}" ]; then
      tls_block="$(cat <<EOF
  tls:
  - hosts:
    - ${ingress_host}
    secretName: ${tls_secret}
EOF
)"
    elif [ "${AETHER_INGRESS_TLS_ACME:-}" = "1" ]; then
      annot_yaml="  annotations:
    cert-manager.io/cluster-issuer: \"${AETHER_INGRESS_ACME_ISSUER:-letsencrypt-prod}\""
    fi
    out+="$(cat <<EOF

---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: aether
  namespace: ${ns}
${annot_yaml}
spec:
EOF
)"
    if [ -n "${ingress_class}" ]; then
      out+="  ingressClassName: ${ingress_class}"
    fi
    out+="${tls_block}
  rules:
  - host: ${ingress_host}
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: aether
            port:
              number: 5090
"
  fi
  printf '%s' "${out}"
}

# Try to open firewall ports on the remote host (best-effort).
aether_deploy_try_open_firewall() {
  local ssh_fn="${1:?}" # name of function: ssh_cmd
  local expose="${2:-${AETHER_EXPOSE:-nodeport}}"
  local node_port="${3:-${AETHER_NODE_PORT:-30090}}"
  [ "${AETHER_OPEN_FIREWALL:-}" = "1" ] || return 0
  local ports=""
  case "${expose}" in
    ingress) ports="80 443" ;;
    both) ports="80 443 ${node_port}" ;;
    *) ports="${node_port}" ;;
  esac
  "${ssh_fn}" "
    if command -v ufw >/dev/null 2>&1 && sudo -n ufw status >/dev/null 2>&1; then
      for p in ${ports}; do sudo -n ufw allow \${p}/tcp || true; done
      echo ufw_ok
    else
      echo no_ufw
    fi
  " 2>/dev/null || true
}
