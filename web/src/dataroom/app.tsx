import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Label,
} from "@biyard/components";
import type { HostSession } from "../../../shared/platform";
import type { GetMaterialResponse } from "@interview/api-types/GetMaterialResponse";
import type { ListMaterialsResponse } from "@interview/api-types/ListMaterialsResponse";
import type { MaterialStatus } from "@interview/api-types/MaterialStatus";
import type { MaterialSummary } from "@interview/api-types/MaterialSummary";
import type { RegisterMaterialParams } from "@interview/api-types/RegisterMaterialParams";
import type { RegisterMaterialResponse } from "@interview/api-types/RegisterMaterialResponse";
import { dataroomRpcHandler } from "@interview/api-client/handlers/dataroomRpcHandler";
import { queryClient } from "../api/query-client";
import { useI18n } from "../i18n";

export function DataroomApp({ session }: { session?: HostSession }) {
  const { t, locale } = useI18n();
  const workspaceId = session?.workspace.id ?? "";
  const isCompany = session?.user.role === "company";

  const [search, setSearch] = useState("");
  const [selectedMaterialId, setSelectedMaterialId] = useState<string | null>(null);
  const [isRegisterOpen, setIsRegisterOpen] = useState(false);

  // Form state
  const [title, setTitle] = useState("");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [fileContent, setFileContent] = useState("");
  const [formError, setFormError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  const materialsQuery = useQuery({
    queryKey: ["dataroom", workspaceId, "materials", search.trim()],
    queryFn: async () => {
      const response = await dataroomRpcHandler({
        workspaceId,
        method: "list_materials",
        params: { search: search.trim() || null },
      });
      return response.result as ListMaterialsResponse;
    },
    enabled: !!workspaceId,
  });

  const materialDetailQuery = useQuery({
    queryKey: ["dataroom", workspaceId, "material", selectedMaterialId],
    queryFn: async () => {
      if (!selectedMaterialId) return null;
      const response = await dataroomRpcHandler({
        workspaceId,
        method: "get_material",
        params: { materialId: selectedMaterialId },
      });
      return response.result as GetMaterialResponse;
    },
    enabled: !!workspaceId && !!selectedMaterialId,
  });

  const registerMutation = useMutation({
    mutationFn: async (params: RegisterMaterialParams) => {
      const response = await dataroomRpcHandler({
        workspaceId,
        method: "register_material",
        params,
      });
      return response.result as RegisterMaterialResponse;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: ["dataroom", workspaceId, "materials"],
      });
      setIsRegisterOpen(false);
      setTitle("");
      setSelectedFile(null);
      setFileContent("");
      setFormError(null);
      setSuccessMessage(t.registerSuccess);
    },
    onError: (error) => {
      // Preserve title and selected file state on error so user does not lose input
      setFormError(error instanceof Error ? error.message : t.registerError);
    },
  });

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) {
      setSelectedFile(null);
      setFileContent("");
      return;
    }

    const lowerName = file.name.toLowerCase();
    if (!lowerName.endsWith(".txt") && !lowerName.endsWith(".md")) {
      setFormError(t.fileRequired);
      setSelectedFile(null);
      setFileContent("");
      return;
    }

    if (file.size > 256 * 1024) {
      setFormError(t.fileTooLarge);
      setSelectedFile(null);
      setFileContent("");
      return;
    }

    setFormError(null);
    setSelectedFile(file);

    const reader = new FileReader();
    reader.onload = () => {
      setFileContent(reader.result as string);
    };
    reader.onerror = () => {
      setFormError(t.fileRequired);
    };
    reader.readAsText(file, "UTF-8");
  };

  const handleRegisterSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) {
      setFormError(t.titleRequired);
      return;
    }
    if (!selectedFile || !fileContent) {
      setFormError(t.fileRequired);
      return;
    }
    setFormError(null);
    registerMutation.mutate({
      title: title.trim(),
      fileName: selectedFile.name,
      content: fileContent,
    });
  };

  const renderStatusBadge = (status: MaterialStatus) => {
    switch (status) {
      case "ready":
        return <Badge variant="success">{t.statusReady}</Badge>;
      case "processing":
        return <Badge variant="warning">{t.statusProcessing}</Badge>;
      case "failed":
        return <Badge variant="danger">{t.statusFailed}</Badge>;
    }
  };

  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return new Intl.DateTimeFormat(locale === "ko" ? "ko-KR" : "en-US", {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(date);
    } catch {
      return dateStr;
    }
  };

  const materials: MaterialSummary[] = materialsQuery.data?.materials ?? [];

  return (
    <div className="space-y-6">
      {/* Header and Controls */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-heading-4 font-semibold">{t.materials}</h1>
          <p className="text-caption text-muted-foreground">{session?.workspace.name}</p>
        </div>
        {isCompany ? (
          <Button
            variant="primary"
            onClick={() => {
              setFormError(null);
              setSuccessMessage(null);
              setIsRegisterOpen(true);
            }}
          >
            {t.registerMaterial}
          </Button>
        ) : null}
      </div>

      {/* Success Notification */}
      {successMessage ? (
        <div
          role="status"
          className="flex items-center justify-between rounded-lg border border-success/30 bg-success-muted p-4 text-success"
        >
          <span>{successMessage}</span>
          <button
            type="button"
            className="text-body-sm font-semibold hover:underline"
            onClick={() => setSuccessMessage(null)}
          >
            {t.close}
          </button>
        </div>
      ) : null}

      {/* Search Bar */}
      <div className="max-w-md">
        <label htmlFor="material-search" className="sr-only">
          {t.searchMaterials}
        </label>
        <Input
          id="material-search"
          type="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={t.searchPlaceholder}
        />
      </div>

      {/* Material List Content */}
      {materialsQuery.isPending ? (
        <p role="status" className="p-6 text-center text-muted-foreground">
          {t.loading}
        </p>
      ) : materialsQuery.isError ? (
        <Card className="border-destructive/30 bg-destructive-muted p-6">
          <div role="alert" className="space-y-4">
            <p className="text-destructive">{t.fetchError}</p>
            <Button variant="outline" onClick={() => materialsQuery.refetch()}>
              {t.retry}
            </Button>
          </div>
        </Card>
      ) : materials.length === 0 ? (
        <Card className="p-8 text-center text-muted-foreground">
          <p>{search.trim() ? t.noSearchResults : t.noMaterials}</p>
        </Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {materials.map((material) => (
            <Card
              key={material.id}
              className="flex flex-col justify-between transition-shadow hover:shadow-md"
            >
              <CardHeader className="space-y-2">
                <div className="flex items-start justify-between gap-2">
                  <CardTitle className="text-heading-5 font-semibold leading-snug">
                    {material.title}
                  </CardTitle>
                  {renderStatusBadge(material.status)}
                </div>
                <p className="text-caption text-muted-foreground break-all">{material.fileName}</p>
              </CardHeader>
              <CardContent className="space-y-4 pt-0">
                <p className="text-caption text-muted-foreground">
                  {t.createdAt}: {formatDate(material.createdAt)}
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full"
                  onClick={() => setSelectedMaterialId(material.id)}
                >
                  {t.viewContent}
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Material Detail Modal */}
      {selectedMaterialId ? (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="material-detail-title"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
        >
          <div className="flex max-h-[90vh] w-full max-w-2xl flex-col rounded-lg border border-border bg-card shadow-lg">
            <div className="flex items-center justify-between border-b border-border p-6">
              <h2 id="material-detail-title" className="text-heading-5 font-semibold">
                {materialDetailQuery.data?.material.title ?? t.materialDetail}
              </h2>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setSelectedMaterialId(null)}
                aria-label={t.close}
              >
                ✕
              </Button>
            </div>

            <div className="flex-1 overflow-y-auto p-6 space-y-4">
              {materialDetailQuery.isPending ? (
                <p role="status" className="p-6 text-center text-muted-foreground">
                  {t.loading}
                </p>
              ) : materialDetailQuery.isError ? (
                <div role="alert" className="space-y-4 rounded-lg bg-destructive-muted p-4">
                  <p className="text-destructive">{t.fetchDetailError}</p>
                  <Button variant="outline" onClick={() => materialDetailQuery.refetch()}>
                    {t.retry}
                  </Button>
                </div>
              ) : materialDetailQuery.data?.material ? (
                <>
                  <div className="flex flex-wrap items-center gap-4 rounded-md bg-secondary/50 p-3 text-caption">
                    <div>
                      <span className="font-semibold">{t.fileName}:</span>{" "}
                      <span>{materialDetailQuery.data.material.fileName}</span>
                    </div>
                    <div>
                      <span className="font-semibold">{t.status}:</span>{" "}
                      {renderStatusBadge(materialDetailQuery.data.material.status)}
                    </div>
                    <div>
                      <span className="font-semibold">{t.createdAt}:</span>{" "}
                      <span>{formatDate(materialDetailQuery.data.material.createdAt)}</span>
                    </div>
                  </div>

                  <div>
                    <Label className="mb-2 block">{t.content}</Label>
                    <pre className="max-h-96 overflow-auto rounded-md border border-border bg-muted/20 p-4 font-mono text-body-sm whitespace-pre-wrap break-words">
                      {materialDetailQuery.data.material.content}
                    </pre>
                  </div>
                </>
              ) : null}
            </div>

            <div className="flex justify-end border-t border-border p-4">
              <Button variant="outline" onClick={() => setSelectedMaterialId(null)}>
                {t.close}
              </Button>
            </div>
          </div>
        </div>
      ) : null}

      {/* Material Registration Modal (Company Only) */}
      {isRegisterOpen && isCompany ? (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="register-modal-title"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
        >
          <div className="w-full max-w-lg rounded-lg border border-border bg-card shadow-lg">
            <div className="flex items-center justify-between border-b border-border p-6">
              <h2 id="register-modal-title" className="text-heading-5 font-semibold">
                {t.registerMaterial}
              </h2>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setIsRegisterOpen(false)}
                aria-label={t.close}
              >
                ✕
              </Button>
            </div>

            <form onSubmit={handleRegisterSubmit} className="p-6 space-y-4">
              {formError ? (
                <div role="alert" className="rounded-md bg-destructive-muted p-3 text-destructive">
                  {formError}
                </div>
              ) : null}

              <div className="space-y-2">
                <Label htmlFor="material-title">{t.materialTitle}</Label>
                <Input
                  id="material-title"
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder={t.materialTitlePlaceholder}
                  disabled={registerMutation.isPending}
                  required
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="material-file">{t.materialFile}</Label>
                <input
                  id="material-file"
                  type="file"
                  accept=".txt,.md"
                  onChange={handleFileChange}
                  disabled={registerMutation.isPending}
                  className="block w-full text-body-sm file:mr-4 file:rounded-md file:border-0 file:bg-secondary file:px-4 file:py-2 file:text-body-sm file:font-semibold file:text-secondary-foreground hover:file:bg-secondary/80 cursor-pointer"
                  required
                />
                <p className="text-caption text-muted-foreground">{t.fileHelp}</p>
                {selectedFile ? (
                  <p className="text-caption font-medium text-foreground">
                    {selectedFile.name} ({(selectedFile.size / 1024).toFixed(1)} KB)
                  </p>
                ) : null}
              </div>

              <div className="flex justify-end gap-3 pt-4 border-t border-border">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setIsRegisterOpen(false)}
                  disabled={registerMutation.isPending}
                >
                  {t.cancel}
                </Button>
                <Button
                  type="submit"
                  variant="primary"
                  disabled={registerMutation.isPending || !title.trim() || !selectedFile}
                >
                  {registerMutation.isPending ? t.registering : t.registerAction}
                </Button>
              </div>
            </form>
          </div>
        </div>
      ) : null}
    </div>
  );
}
