// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useCallback, useEffect, useState } from 'react';
import { Fingerprint, KeyRound, LogIn, Shield } from 'lucide-react';
import { apiFetchAuthSettings, type AuthSettingsPayload } from '../utils/api';
import { copyToClipboard } from '../utils/clipboard';
import GlassSection from './GlassSection';
import Badge from './Badge';
import PageLoading from './PageLoading';

function ConfigRow({ label, value }: { label: string; value: string | null | undefined }) {
  if (!value) return null;
  return (
    <div className="flex flex-col gap-0.5 sm:flex-row sm:items-start sm:justify-between sm:gap-4 py-2 border-b border-white/5 last:border-0">
      <span className="text-xs text-subtle shrink-0">{label}</span>
      <code className="text-xs text-muted break-all text-right">{value}</code>
    </div>
  );
}

function StatusBadge({ enabled }: { enabled: boolean }) {
  return (
    <Badge
      text={enabled ? 'Configured' : 'Not configured'}
      variant={enabled ? 'green' : 'muted'}
    />
  );
}

export default function IdentitySsoPanel() {
  const [settings, setSettings] = useState<AuthSettingsPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    const data = await apiFetchAuthSettings();
    if (!data) {
      setError('Could not load identity settings.');
      setSettings(null);
    } else {
      setSettings(data);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading) {
    return (
      <GlassSection
        accent="purple"
        testId="identity-sso-panel"
        title="Identity & SSO"
        subtitle="OpenID Connect, SAML, and Active Directory configuration (read-only)."
        icon={<Fingerprint className="h-5 w-5 text-lavender" />}
      >
        <PageLoading label="Loading identity settings…" />
      </GlassSection>
    );
  }

  if (error || !settings) {
    return (
      <GlassSection
        accent="purple"
        testId="identity-sso-panel"
        title="Identity & SSO"
        subtitle="OpenID Connect, SAML, and Active Directory configuration (read-only)."
        icon={<Fingerprint className="h-5 w-5 text-lavender" />}
      >
        <p className="text-sm text-danger">{error ?? 'Unknown error'}</p>
      </GlassSection>
    );
  }

  const metadataUrl = settings.saml.metadata_path
    ? `${window.location.origin}${settings.saml.metadata_path}`
    : null;

  return (
    <div className="space-y-6" data-testid="identity-sso-panel">
      <GlassSection
        accent="neutral"
        testId="identity-session-panel"
        title="Session"
        subtitle="JWT session cookies issued after OIDC, SAML, or LDAP login."
        icon={<KeyRound className="h-5 w-5 text-muted" />}
        actions={<StatusBadge enabled={settings.session.session_secret_configured} />}
      >
        <ConfigRow label="Cookie" value={settings.session.cookie_name} />
        <ConfigRow
          label="Redis (HA)"
          value={settings.session.redis_configured ? 'enabled' : 'in-memory (single node)'}
        />
        <ConfigRow label="JWT TTL" value={`${settings.session.jwt_session_hours} hours`} />
        <p className="mt-3 text-xs text-subtle">{settings.session.note}</p>
      </GlassSection>

      <GlassSection
        accent="blue"
        testId="identity-oidc-panel"
        title="OpenID Connect"
        subtitle="Authorization code + PKCE; discovery, callback, and JWT session issuance."
        icon={<LogIn className="h-5 w-5 text-primary" />}
        actions={<StatusBadge enabled={settings.oidc.enabled} />}
      >
        <ConfigRow label="Issuer" value={settings.oidc.issuer} />
        <ConfigRow label="Client ID" value={settings.oidc.client_id} />
        <ConfigRow label="Redirect URI" value={settings.oidc.redirect_uri} />
        <ConfigRow label="Discovery" value={settings.oidc.discovery} />
        <ConfigRow label="Login" value={settings.oidc.login_path} />
        <ConfigRow label="Callback" value={settings.oidc.callback_path} />
        <ConfigRow label="Flow" value={(settings.oidc.flow ?? []).join(', ')} />
        <ConfigRow label="Groups claim" value={settings.oidc.groups_claim} />
        <ConfigRow label="Default role" value={settings.oidc.default_role} />
        <ConfigRow
          label="Client secret"
          value={settings.oidc.client_secret_configured ? 'configured' : 'not set (public client)'}
        />
        <ConfigRow
          label="Role mapping"
          value={settings.oidc.role_mapping_configured ? 'AETHER_OIDC_ROLE_MAP' : 'default role only'}
        />
        <p className="mt-3 text-xs text-subtle">{settings.oidc.note}</p>
      </GlassSection>

      <GlassSection
        accent="purple"
        testId="identity-saml-panel"
        title="SAML 2.0"
        subtitle="HTTP-Redirect login and HTTP-POST ACS; SP metadata for IdP federation."
        icon={<Shield className="h-5 w-5 text-lavender" />}
        actions={<StatusBadge enabled={settings.saml.enabled} />}
      >
        <ConfigRow label="SP entity ID" value={settings.saml.sp_entity_id} />
        <ConfigRow label="ACS URL" value={settings.saml.acs_url} />
        <ConfigRow label="IdP SSO URL" value={settings.saml.idp_sso_url} />
        <ConfigRow label="IdP entity ID" value={settings.saml.idp_entity_id} />
        <ConfigRow
          label="IdP certificate"
          value={settings.saml.idp_cert_configured ? 'configured (signature verify)' : 'not set'}
        />
        <ConfigRow label="Login" value={settings.saml.login_path} />
        <ConfigRow label="ACS path" value={settings.saml.acs_path} />
        <ConfigRow label="Default role" value={settings.saml.default_role} />
        <ConfigRow
          label="Role mapping"
          value={settings.saml.role_mapping_configured ? 'AETHER_SAML_ROLE_MAP' : 'default role only'}
        />
        {metadataUrl ? (
          <div className="mt-4 flex flex-wrap gap-2">
            <a
              href={settings.saml.metadata_path}
              className="btn-secondary inline-flex items-center gap-2 px-4 py-2 text-sm"
              data-testid="saml-metadata-download"
            >
              Download SP metadata
            </a>
            <button
              type="button"
              className="btn-secondary inline-flex items-center gap-2 px-4 py-2 text-sm"
              data-testid="saml-metadata-copy"
              onClick={() => void copyToClipboard(metadataUrl)}
            >
              Copy metadata URL
            </button>
          </div>
        ) : null}
        <p className="mt-3 text-xs text-subtle">{settings.saml.note}</p>
      </GlassSection>

      <GlassSection
        accent="neutral"
        testId="identity-ldap-panel"
        title="Active Directory / LDAP"
        subtitle="Username + password bind; issues the same JWT session cookie."
        icon={<Fingerprint className="h-5 w-5 text-success" />}
        actions={<StatusBadge enabled={settings.ldap.enabled} />}
      >
        <ConfigRow label="LDAP URL" value={settings.ldap.url} />
        <ConfigRow label="Base DN" value={settings.ldap.base_dn} />
        <ConfigRow label="Domain" value={settings.ldap.domain} />
        <ConfigRow label="Login" value={settings.ldap.login_path} />
        <ConfigRow label="Default role" value={settings.ldap.default_role} />
        <ConfigRow
          label="Service bind"
          value={settings.ldap.bind_dn_configured ? 'configured' : 'direct user bind'}
        />
        <ConfigRow
          label="Role mapping"
          value={settings.ldap.role_mapping_configured ? 'AETHER_LDAP_ROLE_MAP' : 'default role only'}
        />
        <p className="mt-3 text-xs text-subtle">{settings.ldap.note}</p>
      </GlassSection>
    </div>
  );
}
