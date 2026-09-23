import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Route, Routes } from "react-router-dom";
import { QueryClientProvider } from "@tanstack/react-query";
import { queryClient } from "./api/query-client";
import { LocaleProvider } from "./i18n";
import { SessionGate } from "./auth/session";
import "./styles.css";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <LocaleProvider>
        <BrowserRouter>
          <Routes>
            <Route path="/workspace/:workspaceId/plugins/:pluginId/*" element={<SessionGate />} />
            <Route path="/workspace/:workspaceId/*" element={<SessionGate />} />
            <Route path="*" element={<SessionGate />} />
          </Routes>
        </BrowserRouter>
      </LocaleProvider>
    </QueryClientProvider>
  </React.StrictMode>,
);
