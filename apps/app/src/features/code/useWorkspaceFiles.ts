import { useCallback, useEffect, useMemo, useState } from "react";
import type { FileEntry } from "@sorycode/contracts";
import { useEngine } from "../../app/providers/EngineProvider";
import { coreClient } from "../../services";

export interface UseWorkspaceFilesResult {
  files: FileEntry[];
  selected: string | null;
  content: string;
  loading: boolean;
  error: string | null;
  reload: () => Promise<void>;
  open: (path: string) => Promise<void>;
}

/**
 * File listing for the active workspace.
 *
 * Every consumer shares the same source of truth (the engine workspace), so
 * switching workspace re-lists and drops the selection that pointed into the
 * previous directory.
 */
export function useWorkspaceFiles(): UseWorkspaceFilesResult {
  const { workspace } = useEngine();
  const workspacePath = workspace?.path ?? null;
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setFiles(await coreClient.listFiles());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    setSelected(null);
    setContent("");
    void reload();
  }, [reload, workspacePath]);

  const open = useCallback(async (path: string) => {
    setSelected(path);
    setLoading(true);
    setError(null);
    try {
      setContent(await coreClient.readFile(path));
    } catch (cause) {
      setContent("");
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  return { files, selected, content, loading, error, reload, open };
}

export function useFilteredFiles(files: FileEntry[], query: string): FileEntry[] {
  return useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return files;
    return files.filter((file) => file.path.toLowerCase().includes(needle));
  }, [files, query]);
}
