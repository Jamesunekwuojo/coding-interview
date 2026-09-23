import { useQuery } from "@tanstack/react-query";
import { scopedKey } from "@interview/plugin-sdk";
import type { PluginProps } from "@interview/plugin-sdk/react";
import type { ReviewHealthResponse } from "@interview/api-types/ReviewHealthResponse";

export const App: React.FC<PluginProps> = ({ host, context }) => {
  const text =
    context.locale === "ko"
      ? {
          title: "Review Plugin",
          loading: "연결을 확인하고 있습니다.",
          error: "Plugin API에 연결하지 못했습니다.",
          status: "Plugin API 연결 상태",
        }
      : {
          title: "Review Plugin",
          loading: "Checking the connection.",
          error: "Could not connect to the Plugin API.",
          status: "Plugin API connection",
        };
  const health = useQuery({
    queryKey: scopedKey(context, "review", "health"),
    queryFn: () => host.call<ReviewHealthResponse>("health"),
  });

  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <h1 className="text-heading-4 font-semibold">{text.title}</h1>
      {health.isPending ? <p role="status">{text.loading}</p> : null}
      {health.isError ? <p role="alert">{text.error}</p> : null}
      {health.data ? (
        <p className="text-muted-foreground">
          {text.status}: {health.data.status}
        </p>
      ) : null}
    </section>
  );
};
