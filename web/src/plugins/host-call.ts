import { ApiError } from "@interview/api-client/runtime/client";
import { dataroomRpcHandler } from "@interview/api-client/handlers/dataroomRpcHandler";
import { pluginRpcHandler } from "@interview/api-client/handlers/pluginRpcHandler";
import type { PluginHost } from "@interview/plugin-sdk";

export function createPluginCall({
  pluginId,
  workspaceId,
  signal,
  onUnauthorized,
}: {
  pluginId: string;
  workspaceId(): string;
  signal: AbortSignal;
  onUnauthorized(): void;
}): PluginHost["call"] {
  return async <T>(
    method: string,
    params?: unknown,
    options?: { target?: "dataroom"; signal?: AbortSignal },
  ): Promise<T> => {
    const ensureActive = () => {
      signal.throwIfAborted();
      options?.signal?.throwIfAborted();
    };
    ensureActive();
    const request = { workspaceId: workspaceId(), method, params: params ?? null };
    try {
      const result =
        options?.target === "dataroom"
          ? await dataroomRpcHandler(request)
          : await pluginRpcHandler({ ...request, pluginId });
      ensureActive();
      return result.result as T;
    } catch (error) {
      ensureActive();
      if (error instanceof ApiError) {
        if (error.status === 401) onUnauthorized();
        throw Object.assign(new Error(error.message), {
          status: error.status,
          kind: error.body.kind,
        });
      }
      throw error;
    }
  };
}
