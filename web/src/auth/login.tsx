import { useState, type FormEvent } from "react";
import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";
import { Button, Input } from "@biyard/components";
import { queryClient } from "../api/query-client";
import { useI18n } from "../i18n";
import { authAdapter } from "./adapter";

export function Login() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const login = useMutation({
    mutationFn: () => authAdapter.login({ email, password }),
    onSuccess: async (user) => {
      await queryClient.cancelQueries();
      queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== "session" });
      queryClient.setQueryData(["session"], user);
      navigate("/", { replace: true });
    },
  });
  function submit(event: FormEvent) {
    event.preventDefault();
    login.mutate();
  }
  function fillAccount(email: string) {
    setEmail(email);
    setPassword("dataroom");
  }
  return (
    <main className="mx-auto flex min-h-screen max-w-md items-center p-6">
      <form
        onSubmit={submit}
        className="w-full space-y-6 rounded-lg border border-border bg-card p-6"
      >
        <h1 className="text-heading-3 font-semibold">{t.appName}</h1>
        <p className="text-muted-foreground">{t.demoNotice}</p>
        <div className="grid grid-cols-2 gap-2">
          <Button
            type="button"
            variant="outline"
            onClick={() => fillAccount("company@lighthouse.test")}
          >
            {t.companyAccount}
          </Button>
          <Button
            type="button"
            variant="outline"
            onClick={() => fillAccount("investor@lighthouse.test")}
          >
            {t.investorAccount}
          </Button>
        </div>
        <label className="flex flex-col gap-2">
          {t.email}
          <Input
            type="email"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            autoComplete="username"
            required
            disabled={login.isPending}
          />
        </label>
        <label className="flex flex-col gap-2">
          {t.password}
          <Input
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            autoComplete="current-password"
            required
            disabled={login.isPending}
          />
        </label>
        {login.isError ? (
          <p role="alert" className="text-destructive">
            {t.loginError}
          </p>
        ) : null}
        <Button type="submit" disabled={login.isPending} className="w-full">
          {login.isPending ? t.loading : t.login}
        </Button>
      </form>
    </main>
  );
}
