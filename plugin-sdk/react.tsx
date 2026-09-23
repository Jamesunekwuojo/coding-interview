import { Component, type ComponentType, type ReactNode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { isPluginRpcError, type Plugin, type PluginContext, type PluginHost } from "./index";
export interface PluginProps {
  host: PluginHost;
  context: PluginContext;
}
class RenderBoundary extends Component<
  { children: ReactNode; locale: string },
  { failed: boolean }
> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? (
      <p role="alert">
        {this.props.locale === "ko"
          ? "플러그인 화면을 표시하지 못했습니다. 페이지를 새로고침해주세요."
          : "The plugin could not render. Reload the page."}
      </p>
    ) : (
      this.props.children
    );
  }
}
export function createReactPlugin(App: ComponentType<PluginProps>): Plugin {
  let root: Root | null = null;
  let client: QueryClient | null = null;
  let host: PluginHost | null = null;
  function render(context: PluginContext) {
    if (!root || !client || !host) return;
    root.render(
      <QueryClientProvider client={client}>
        <RenderBoundary locale={context.locale}>
          <App host={host} context={context} />
        </RenderBoundary>
      </QueryClientProvider>,
    );
  }
  return {
    mount(element, nextHost) {
      host = nextHost;
      client = new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 15_000,
            retry: (count, error) => !(isPluginRpcError(error) && error.status < 500) && count < 1,
          },
          mutations: { retry: false },
        },
      });
      root = createRoot(element);
      render({ ...host.context });
    },
    update(context) {
      render({ ...context });
    },
    unmount() {
      const previous = root;
      root = null;
      void client?.cancelQueries();
      client?.clear();
      client = null;
      host = null;
      if (previous) queueMicrotask(() => previous.unmount());
    },
  };
}
