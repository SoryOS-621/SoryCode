import { useState } from "react";
import {
  Badge,
  Button,
  Dialog,
  Field,
  Icon,
  Input,
  Panel,
  PanelBody,
  PanelHeader,
  Select,
  StatusDot,
  useTheme,
  type ThemeMode,
} from "@sorycode/ui";
import type { ProviderInfo } from "@sorycode/contracts";
import { useEngine } from "../../app/providers/EngineProvider";

const THEMES: { id: ThemeMode; label: string; icon: "sun" | "moon" | "monitor" }[] = [
  { id: "light", label: "Light", icon: "sun" },
  { id: "dark", label: "Dark", icon: "moon" },
  { id: "system", label: "System", icon: "monitor" },
];

export function SettingsView() {
  const {
    providers,
    selection,
    workspace,
    appInfo,
    selectProvider,
    selectModel,
    connectProvider,
    error,
  } = useEngine();
  const { mode, setMode } = useTheme();
  const [target, setTarget] = useState<ProviderInfo | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [saving, setSaving] = useState(false);

  const current = providers.find((provider) => provider.id === selection?.provider);

  const connect = async () => {
    if (!target) return;
    setSaving(true);
    try {
      await connectProvider(target.id, apiKey);
      setTarget(null);
      setApiKey("");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="sory-view">
      {error ? <div className="sory-error">{error}</div> : null}

      <div className="sory-grid sory-grid-settings">
        <Panel>
          <PanelHeader
            title="Model provider"
            description="Select the provider and model used by the agent."
            actions={selection ? <Badge tone="accent">{selection.model}</Badge> : null}
          />
          <PanelBody>
            <div className="sory-col">
              <Field label="Provider" htmlFor="settings-provider">
                <Select
                  id="settings-provider"
                  value={selection?.provider ?? ""}
                  onChange={(event) => void selectProvider(event.target.value)}
                >
                  {providers.map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.name} — {provider.description}
                      {provider.configured ? "" : " (not configured)"}
                    </option>
                  ))}
                </Select>
              </Field>

              <Field
                label="Model"
                htmlFor="settings-model"
                hint={current ? `${current.models.length} models available` : undefined}
              >
                <Select
                  id="settings-model"
                  value={selection?.model ?? ""}
                  onChange={(event) => void selectModel(event.target.value)}
                >
                  {(selection?.models ?? []).map((model) => (
                    <option key={model} value={model}>
                      {model}
                    </option>
                  ))}
                </Select>
              </Field>

              <Field label="Custom model" htmlFor="settings-custom-model" hint="Enter a model id and press Enter.">
                <Input
                  id="settings-custom-model"
                  placeholder="provider/model-name"
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      void selectModel((event.target as HTMLInputElement).value);
                    }
                  }}
                />
              </Field>
            </div>
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader title="Appearance" description="Themes apply to every SoryCode surface." />
          <PanelBody>
            <div className="sory-inline">
              {THEMES.map((theme) => (
                <Button
                  key={theme.id}
                  variant={mode === theme.id ? "primary" : "secondary"}
                  icon={theme.icon}
                  onClick={() => setMode(theme.id)}
                >
                  {theme.label}
                </Button>
              ))}
            </div>
          </PanelBody>
        </Panel>
      </div>

      <Panel>
        <PanelHeader
          title="Providers"
          description="Connect a provider to unlock its models. Free providers work without a paid key."
        />
        <PanelBody>
          <div className="sory-grid sory-grid-providers">
            {providers.map((provider) => (
              <div
                key={provider.id}
                className={
                  provider.id === selection?.provider ? "sory-card sory-card-active" : "sory-card"
                }
              >
                <div className="sory-card-head">
                  <StatusDot
                    tone={provider.configured ? "success" : "warning"}
                    label={provider.configured ? "configured" : "not configured"}
                  />
                  <strong className="sory-card-title">{provider.name}</strong>
                  {provider.freeOnly ? <Badge tone="info">free</Badge> : null}
                  {provider.id === selection?.provider ? <Badge tone="accent">active</Badge> : null}
                </div>
                <div className="sory-card-text sory-card-text-tight">
                  {provider.description} · {provider.models.length} models
                </div>
                <div className="sory-card-actions">
                  <Button
                    size="sm"
                    variant="secondary"
                    icon="check"
                    onClick={() => void selectProvider(provider.id)}
                  >
                    Use
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    icon="key"
                    onClick={() => {
                      setTarget(provider);
                      setApiKey("");
                    }}
                  >
                    {provider.configured ? "Update key" : "Connect"}
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </PanelBody>
      </Panel>

      <Panel>
        <PanelHeader title="About" description="Paths and version information." />
        <PanelBody>
          <div className="sory-grid sory-grid-info">
            <InfoRow icon="sparkles" label="Version" value={appInfo?.version ?? "—"} />
            <InfoRow icon="folder" label="Workspace" value={workspace?.path ?? "—"} />
            <InfoRow icon="sessions" label="Data" value={appInfo?.dataPath ?? "—"} />
            <InfoRow icon="key" label="Credentials" value={appInfo?.credentialsPath ?? "—"} />
          </div>
        </PanelBody>
      </Panel>

      <Dialog
        open={target !== null}
        onClose={() => setTarget(null)}
        title={`Connect ${target?.name ?? ""}`}
        footer={
          <>
            <Button variant="ghost" onClick={() => setTarget(null)}>
              Cancel
            </Button>
            <Button variant="primary" loading={saving} onClick={() => void connect()}>
              Save key
            </Button>
          </>
        }
      >
        <div className="sory-col">
          <p className="sory-note">
            {target?.description}. The key is stored in <code>~/.sorycode/credentials.json</code> with
            owner-only permissions.
          </p>
          {target?.freeOnly ? (
            <p className="sory-note-info">
              This provider offers free models. You can leave the key empty to use anonymous access.
            </p>
          ) : null}
          <Field label="API key" htmlFor="api-key" hint="Environment variables take precedence over this file.">
            <Input
              id="api-key"
              type="password"
              value={apiKey}
              onChange={(event) => setApiKey(event.target.value)}
              placeholder="sk-…"
              autoFocus
            />
          </Field>
        </div>
      </Dialog>
    </div>
  );
}

function InfoRow({ icon, label, value }: { icon: "sparkles" | "folder" | "sessions" | "key"; label: string; value: string }) {
  return (
    <div className="sory-info-row">
      <Icon name={icon} size={14} />
      <span className="sory-info-label">{label}</span>
      <span className="sory-info-value" title={value}>
        {value}
      </span>
    </div>
  );
}
