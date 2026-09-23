import { Button } from "@biyard/components";
import { useI18n } from "../i18n";

export function ConnectionError({ retry }: { retry(): void }) {
  const { t } = useI18n();
  return (
    <div
      role="alert"
      className="m-8 flex flex-col items-start gap-4 rounded-lg border border-border bg-card p-6"
    >
      <p>{t.connectionError}</p>
      <Button variant="outline" onClick={retry}>
        {t.retry}
      </Button>
    </div>
  );
}
