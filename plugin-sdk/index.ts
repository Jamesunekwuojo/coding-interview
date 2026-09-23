import type { Viewer } from "../shared/platform";
export type Locale = "ko" | "en";
export interface PluginContext {
  user: Viewer;
  workspaceId: string;
  locale: Locale;
  location: string;
}
export interface PluginHost {
  context: PluginContext;
  call<T = unknown>(
    method: string,
    params?: unknown,
    options?: { target?: "dataroom"; signal?: AbortSignal },
  ): Promise<T>;
  navigate(path: string): void;
}
export interface Plugin {
  mount(element: HTMLElement, host: PluginHost): void;
  update(context: PluginContext): void;
  unmount(): void;
}
export interface PluginManifest {
  id: string;
  name: string;
  contract: 1;
  entry: string;
  styles: string[];
}
export interface PluginRpcError extends Error {
  status: number;
  kind: string;
}
export function isPluginRpcError(error: unknown): error is PluginRpcError {
  return (
    typeof error === "object" &&
    error !== null &&
    "status" in error &&
    typeof error.status === "number" &&
    "kind" in error &&
    typeof error.kind === "string" &&
    "message" in error &&
    typeof error.message === "string"
  );
}
export const scopedKey = (context: PluginContext, ...parts: readonly unknown[]) =>
  [context.user.id, context.workspaceId, ...parts] as const;
