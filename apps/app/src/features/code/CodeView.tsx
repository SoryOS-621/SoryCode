import { useState } from "react";
import {
  Badge,
  Breadcrumbs,
  Button,
  CodeBlock,
  EmptyState,
  Icon,
  IconButton,
  Panel,
  PanelBody,
  PanelHeader,
  SearchInput,
  Spinner,
  SplitLayout,
} from "@sorycode/ui";
import { useEngine } from "../../app/providers/EngineProvider";
import { useFilteredFiles, useWorkspaceFiles } from "./useWorkspaceFiles";

export function CodeView() {
  const { workspace } = useEngine();
  const workspaceFiles = useWorkspaceFiles();
  const [query, setQuery] = useState("");
  const files = useFilteredFiles(workspaceFiles.files, query);

  return (
    <div className="sory-view">
      <div className="sory-view-header">
        <Breadcrumbs items={[workspace?.name ?? "Workspace", "Code"]} />
        <span className="sory-spacer" />
        {workspace?.isGit && workspace.gitBranch ? (
          <Badge tone="info">
            <Icon name="git" size={11} /> {workspace.gitBranch}
          </Badge>
        ) : null}
        <IconButton icon="refresh" label="Refresh files" onClick={() => void workspaceFiles.reload()} />
      </div>

      <SplitLayout
        left={
          <Panel className="sory-fill">
            <PanelHeader
              title="Files"
              description={`${workspaceFiles.files.length} entries`}
              actions={
                <div className="sory-slot-tree">
                  <SearchInput
                    placeholder="Filter files…"
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                  />
                </div>
              }
            />
            <PanelBody className="sory-panel-body-tight">
              {workspaceFiles.loading && workspaceFiles.files.length === 0 ? (
                <div className="sory-loading">
                  <Spinner />
                </div>
              ) : files.length === 0 ? (
                <EmptyState icon="folder" title="No files" description="The workspace is empty or filtered out." />
              ) : (
                <div className="sory-file-tree">
                  {files.map((file) => {
                    const depth = file.path.split("/").length - 1;
                    const active = workspaceFiles.selected === file.path;
                    return (
                      <button
                        key={file.path}
                        type="button"
                        disabled={file.isDir}
                        className={active ? "sory-file sory-file-active" : "sory-file"}
                        style={{ paddingLeft: `${8 + depth * 14}px` }}
                        onClick={() => void workspaceFiles.open(file.path)}
                      >
                        <Icon name={file.isDir ? "folder" : "file"} size={13} />
                        <span>{file.name}</span>
                      </button>
                    );
                  })}
                </div>
              )}
            </PanelBody>
          </Panel>
        }
        right={
          <Panel className="sory-fill">
            <PanelHeader
              title={workspaceFiles.selected ?? "Preview"}
              description={workspaceFiles.selected ? "Read-only preview" : "Select a file to preview it"}
              actions={
                workspaceFiles.selected ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    icon="external"
                    onClick={() => void workspaceFiles.open(workspaceFiles.selected!)}
                  >
                    Reload
                  </Button>
                ) : null
              }
            />
            <PanelBody className="sory-panel-body-md">
              {workspaceFiles.error ? (
                <div className="sory-error">{workspaceFiles.error}</div>
              ) : workspaceFiles.loading ? (
                <div className="sory-loading">
                  <Spinner />
                </div>
              ) : workspaceFiles.selected ? (
                <CodeBlock code={workspaceFiles.content} language={languageOf(workspaceFiles.selected)} maxHeight={640} />
              ) : (
                <EmptyState icon="code" title="No file selected" description="Pick a file from the tree." />
              )}
            </PanelBody>
          </Panel>
        }
      />
    </div>
  );
}

function languageOf(path: string): string {
  const extension = path.split(".").pop()?.toLowerCase();
  const languages: Record<string, string> = {
    rs: "rust",
    ts: "typescript",
    tsx: "tsx",
    js: "javascript",
    jsx: "jsx",
    json: "json",
    md: "markdown",
    toml: "toml",
    css: "css",
    html: "html",
    yml: "yaml",
    yaml: "yaml",
    sh: "bash",
  };
  return extension ? (languages[extension] ?? extension) : "text";
}
