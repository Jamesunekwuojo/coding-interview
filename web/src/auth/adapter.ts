import type { HostSession } from "../../../shared/platform";
import { getSessionHandler } from "@interview/api-client/handlers/getSessionHandler";
import { loginHandler } from "@interview/api-client/handlers/loginHandler";
import { logoutHandler } from "@interview/api-client/handlers/logoutHandler";
import type { LoginRequest } from "@interview/api-types/LoginRequest";

export const authAdapter = {
  async restore(signal?: AbortSignal): Promise<HostSession | null> {
    signal?.throwIfAborted();
    const response = await getSessionHandler();
    signal?.throwIfAborted();
    return response.session;
  },
  async login(input: LoginRequest): Promise<HostSession> {
    return loginHandler(input);
  },
  async logout(): Promise<void> {
    await logoutHandler();
  },
};
