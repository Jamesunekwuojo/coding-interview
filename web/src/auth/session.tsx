import { useQuery } from "@tanstack/react-query";
import { authAdapter } from "./adapter";
import { queryClient } from "../api/query-client";
import { ConnectionError } from "../components/connection-error";
import { useI18n } from "../i18n";
import { Login } from "./login";
import { Shell } from "../components/shell";
export function expireSession() {
  void queryClient.cancelQueries();
  queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== "session" });
  queryClient.setQueryData(["session"], null);
}
export function SessionGate() {
  const { t } = useI18n();
  const session = useQuery({
    queryKey: ["session"],
    queryFn: ({ signal }) => authAdapter.restore(signal),
    refetchInterval: 60_000,
  });
  if (session.isPending)
    return (
      <p className="p-6" role="status">
        {t.loading}
      </p>
    );
  if (session.isError) return <ConnectionError retry={() => session.refetch()} />;
  if (!session.data) return <Login />;
  return <Shell key={session.data.user.id} session={session.data} />;
}
