import { useQuery } from "@tanstack/react-query";
import { listPluginsHandler } from "@interview/api-client/handlers/listPluginsHandler";

export function usePlugins() {
  return useQuery({
    queryKey: ["plugins"],
    queryFn: listPluginsHandler,
  });
}
