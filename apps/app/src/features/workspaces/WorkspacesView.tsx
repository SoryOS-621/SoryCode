import { useEffect, useState } from "react";
import {
  Badge,
  Button,
  EmptyState,
  Field,
  Icon,
  Input,
  Panel,
  PanelBody,
  PanelHeader,
  Spinner,
} from "@sorycode/ui";
import { useEngine } from "../../app/providers/EngineProvider";
import { coreClient } from "../../services";
import { useWorkspaceFiles } from "../code/useWorkspaceFiles";

export function WorkspacesView() {
  const { workspace, openWorkspace } = useEngine();
  const [path, setPath] = useState(workspace?.path ?? "");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const files = useWorkspaceFiles();

  useEffect(() => {
    if (workspace?.path) setPath(workspace.path);
  }, [workspace?.path]);

  const browse = async () => {
    setSaving(true);
    setError(null);
    try {
      const picked = await coreClient.pickWorkspaceFolder();
      if (picked) {
        setPath(picked);
        await openWorkspace(picked);
        await files.reload();
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  };

  const open = async () => {
    if (!path.trim()) return;
    setSaving(true);
    setError(null);
    try {
      await openWorkspace(path.trim());
      await files.reload();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  };

  const topLevel = files.files.filter((file) => !file.path.includes("/"));

  return (
    <div className="sory-view">
      <div className="sory-view-header">
        <div className="sory-view-heading">
          <h1 className="sory-view-title">Workspaces</h1>
          <p className="sory-view-subtitle">
            The workspace is the directory the agent is allowed to inspect and modify.
          </p>
        </div>
      </div>

      <Panel>
        <PanelHeader title="Current workspace" description={workspace?.path} />
        <PanelBody>
          <div className="sory-identity">
            <span className="sory-identity-icon">
              <Icon name="folder" size={18} />
            </span>
            <div className="sory-identity-main">
              <div className="sory-identity-name">{workspace?.name ?? "No workspace"}</div>
              <div className="sory-identity-path">{workspace?.path}</div>
            </div>
            {workspace?.isGit && workspace.gitBranch ? (
              <Badge tone="info">
                <Icon name="git" size={11} /> {workspace.gitBranch}
              </Badge>
            ) : (
              <Badge>no git</Badge>
            )}
            <Badge tone="accent">{files.files.length} entries</Badge>
          </div>

          <div className="sory-inline-end">
            <div className="sory-grow">
              <Field label="Open another workspace" htmlFor="workspace-path" error={error ?? undefined}>
                <Input
                  id="workspace-path"
                  value={path}
                  onChange={(event) => setPath(event.target.value)}
                  placeholder="/absolute/path/to/project"
                />
              </Field>
            </div>
            <Button
              variant="secondary"
              loading={saving}
              icon="folder"
              onClick={() => void browse()}
            >
              Browse…
            </Button>
            <Button variant="primary" loading={saving} icon="folder" onClick={() => void open()}>
              Open
            </Button>
          </div>
        </PanelBody>
      </Panel>

      <Panel className="sory-fill">
        <PanelHeader
          title="Workspace root"
          description="Top-level entries (read-only overview)"
          actions={
            <Button size="sm" variant="ghost" icon="refresh" onClick={() => void files.reload()}>
              Refresh
            </Button>
          }
        />
        <PanelBody className="sory-panel-body-compact">
          {files.loading ? (
            <div className="sory-loading">
              <Spinner />
            </div>
          ) : topLevel.length === 0 ? (
            <EmptyState icon="folder" title="Empty workspace" description="No files were found." />
          ) : (
            <div className="sory-grid sory-grid-tiles sory-grid-tight">
              {topLevel.map((entry) => (
                <div key={entry.path} className="sory-row sory-row-tight">
                  <Icon name={entry.isDir ? "folder" : "file"} size={14} />
                  <span className="sory-truncate">{entry.name}</span>
                </div>
              ))}
            </div>
          )}
        </PanelBody>
      </Panel>
    </div>
  );
}
