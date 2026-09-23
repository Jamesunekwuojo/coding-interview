import type { ApiErrorBody } from "../types/ApiErrorBody";

export class ApiError extends Error {
  constructor(
    public status: number,
    public body: ApiErrorBody,
  ) {
    super(body.message);
  }
}

export async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  if (init?.body) headers.set("content-type", "application/json");
  const response = await fetch(path, { ...init, headers, credentials: "same-origin" });
  if (!response.ok) {
    const fallback = { kind: "request_failed", message: "The request could not be completed." };
    const body: unknown = await response.json().catch(() => fallback);
    const error =
      body &&
      typeof body === "object" &&
      "kind" in body &&
      "message" in body &&
      typeof body.kind === "string" &&
      typeof body.message === "string"
        ? { kind: body.kind, message: body.message }
        : fallback;
    throw new ApiError(response.status, error);
  }
  if (response.status === 204) return undefined as T;
  const text = await response.text();
  return text ? (JSON.parse(text) as T) : (undefined as T);
}

function url(path: string, query?: object): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query ?? {})) {
    if (value !== null && value !== undefined) params.set(key, String(value));
  }
  const encoded = params.toString();
  return encoded ? `${path}?${encoded}` : path;
}
export function apiGet<T>(path: string, query?: object): Promise<T> {
  return request(url(path, query));
}
export function apiDelete<T>(path: string, query?: object): Promise<T> {
  return request(url(path, query), { method: "DELETE" });
}
export function apiPost<T>(path: string, body?: unknown, query?: object): Promise<T> {
  return request(url(path, query), { method: "POST", body: JSON.stringify(body) });
}
export function apiPut<T>(path: string, body?: unknown, query?: object): Promise<T> {
  return request(url(path, query), { method: "PUT", body: JSON.stringify(body) });
}
export function apiPatch<T>(path: string, body?: unknown, query?: object): Promise<T> {
  return request(url(path, query), { method: "PATCH", body: JSON.stringify(body) });
}
