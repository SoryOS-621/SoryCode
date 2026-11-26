import { useCallback, useEffect, useState } from "react";
import {
  Badge,
  Button,
  Dialog,
  EmptyState,
  Field,
  Icon,
  Input,
  Panel,
  PanelBody,
  PanelHeader,
  Spinner,
} from "@sorycode/ui";
import type { ProjectSummary } from "@sorycode/contracts";
import { useEngine } from "../../app/providers/EngineProvider";
import { coreClient } from "../../services";

export function ProjectsView() {
  const { workspace, refresh } = useEngine();
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [name, setName] = useState("");
  const [rootPath, setRootPath] = useState(workspace?.path ?? "");
  const [saving, setSaving] = useState(false);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      setProjects(await coreClient.listProjects());
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    if (!rootPath && workspace?.path) setRootPath(workspace.path);
  }, [workspace, rootPath]);

  const create = async () => {
    if (!name.trim()) return;
    setSaving(true);
    try {
      const project = await coreClient.createProject(name.trim(), rootPath.trim());
      setProjects((current) => [project, ...current]);
      setDialogOpen(false);
      setName("");
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="sory-view">
      <div className="sory-view-header">
        <div className="sory-view-heading">
          <h1 className="sory-view-title">Projects</h1>
          <p className="sory-view-subtitle">
            Register repositories and open them as workspaces.
          </p>
        </div>
        <Button size="sm" variant="secondary" icon="refresh" onClick={() => void reload()}>
          Refresh
        </Button>
        <Button size="sm" variant="primary" icon="plus" onClick={() => setDialogOpen(true)}>
          New project
        </Button>
      </div>

      {error ? <div className="sory-error">{error}</div> : null}

      <Panel className="sory-fill">
        <PanelHeader title="Registered projects" description={`${projects.length} project(s)`} />
        <PanelBody>
          {loading && projects.length === 0 ? (
            <div className="sory-loading">
              <Spinner />
            </div>
          ) : projects.length === 0 ? (
            <EmptyState
              icon="projects"
              title="No projects yet"
              description="Add a project to keep its sessions and tasks together."
              action={
                <Button variant="primary" icon="plus" onClick={() => setDialogOpen(true)}>
                  New project
                </Button>
              }
            />
          ) : (
            <div className="sory-grid sory-grid-cards">
              {projects.map((project) => (
                <div key={project.id} className="sory-card">
                  <div className="sory-card-head">
                    <Icon name="projects" size={15} />
                    <strong className="sory-card-title">{project.name}</strong>
                    <Badge tone="info">project</Badge>
                  </div>
                  <div className="sory-card-meta">{project.rootPath}</div>
                  <div className="sory-card-actions">
                    <Button
                      size="sm"
                      variant="secondary"
                      icon="folder"
                      onClick={() => {
                        void coreClient
                          .openProject(project.id)
                          // The shell switches the active project: re-read the
                          // workspace so the header badge follows immediately.
                          .then(refresh)
                          .catch((cause: unknown) =>
                            setError(cause instanceof Error ? cause.message : String(cause)),
                          );
                      }}
                    >
                      Open
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </PanelBody>
      </Panel>

      <Dialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        title="New project"
        footer={
          <>
            <Button variant="ghost" onClick={() => setDialogOpen(false)}>
              Cancel
            </Button>
            <Button variant="primary" loading={saving} onClick={() => void create()}>
              Create
            </Button>
          </>
        }
      >
        <div className="sory-col">
          <Field label="Project name" htmlFor="project-name">
            <Input
              id="project-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="SoryCode Core"
            />
          </Field>
          <Field label="Root path" htmlFor="project-root" hint="Absolute path to the repository.">
            <div className="sory-inline-end">
              <div className="sory-grow">
                <Input
                  id="project-root"
                  value={rootPath}
                  onChange={(event) => setRootPath(event.target.value)}
                  placeholder="/home/user/project"
                />
              </div>
              <Button
                size="sm"
                variant="secondary"
                icon="folder"
                onClick={() =>
                  void coreClient
                    .pickWorkspaceFolder()
                    .then((picked) => {
                      if (picked) setRootPath(picked);
                    })
                    .catch(() => undefined)
                }
              >
                Browse…
              </Button>
            </div>
          </Field>
        </div>
      </Dialog>
    </div>
  );
}
