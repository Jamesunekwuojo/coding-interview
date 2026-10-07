import React, { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { scopedKey } from "@interview/plugin-sdk";
import type { PluginProps } from "@interview/plugin-sdk/react";
import { Badge, Button, Card, CardContent, CardHeader, CardTitle } from "@biyard/components";
import type { GetReviewResponse } from "@interview/api-types/GetReviewResponse";
import type { GetSummaryResponse } from "@interview/api-types/GetSummaryResponse";
import type { ListCriteriaResponse } from "@interview/api-types/ListCriteriaResponse";
import type { ListReviewsResponse } from "@interview/api-types/ListReviewsResponse";
import type { ReviewCriterion } from "@interview/api-types/ReviewCriterion";
import type { ReviewStatus } from "@interview/api-types/ReviewStatus";
import type { ReviewSummaryItem } from "@interview/api-types/ReviewSummaryItem";

export const App: React.FC<PluginProps> = ({ host, context }) => {
  const isInvestor = context.user.role === "investor";
  const [selectedReviewId, setSelectedReviewId] = useState<string | null>(null);

  const t =
    context.locale === "ko"
      ? {
          pluginTitle: "투자 검토 대시보드",
          pluginSubtitle: "투자자 검토 및 평가",
          companyRestrictedTitle: "투자자 전용 기능",
          companyRestrictedDesc:
            "검토 플러그인은 투자자 전용 기능입니다. 회사 계정은 투자 검토 및 진행 상황을 조회할 수 없습니다.",
          progressSummary: "검토 진행 현황",
          completed: "완료됨",
          remaining: "남음",
          satisfied: "충족",
          needsInformation: "정보 필요",
          criteriaTitle: "검토 기준",
          reviewsTitle: "작성된 검토 목록",
          noReviewsYet: "아직 작성된 검토가 없습니다",
          newInvestorHelp:
            "모든 기준이 검토 대기 상태입니다. 기준을 검토하고 의견을 작성할 수 있습니다.",
          pendingReview: "검토 대기",
          reviewed: "검토 완료",
          viewDetails: "상세 보기",
          close: "닫기",
          loading: "불러오는 중입니다...",
          detailLoading: "상세 정보를 불러오는 중입니다...",
          fetchError: "검토 데이터를 불러오지 못했습니다.",
          fetchDetailError: "검토 상세 정보를 불러오지 못했습니다.",
          retry: "다시 시도",
          reviewDetail: "검토 상세 정보",
          reviewQuestion: "검토 질문",
          opinion: "검토 의견",
          evidence: "증빙 자료",
          noEvidence: "연결된 증빙 자료가 없습니다.",
          fileName: "파일명",
          status: "상태",
          createdAt: "작성일시",
          updatedAt: "수정일시",
          statusReady: "준비 완료",
          statusProcessing: "처리 중",
          statusFailed: "실패",
        }
      : {
          pluginTitle: "Review Dashboard",
          pluginSubtitle: "Investor Review & Evaluation",
          companyRestrictedTitle: "Investor-Only Feature",
          companyRestrictedDesc:
            "The Review Plugin is private to investors. Company users cannot view or submit review progress.",
          progressSummary: "Review Progress",
          completed: "Completed",
          remaining: "Remaining",
          satisfied: "Satisfied",
          needsInformation: "Needs Information",
          criteriaTitle: "Review Criteria",
          reviewsTitle: "Submitted Reviews",
          noReviewsYet: "No reviews submitted yet",
          newInvestorHelp:
            "All criteria remain to be reviewed. You can evaluate each criterion and submit your findings.",
          pendingReview: "Pending Review",
          reviewed: "Reviewed",
          viewDetails: "View Details",
          close: "Close",
          loading: "Loading review data...",
          detailLoading: "Loading review details...",
          fetchError: "Failed to load review data.",
          fetchDetailError: "Failed to load review details.",
          retry: "Retry",
          reviewDetail: "Review Detail",
          reviewQuestion: "Review Question",
          opinion: "Opinion",
          evidence: "Evidence Materials",
          noEvidence: "No evidence materials attached.",
          fileName: "File Name",
          status: "Status",
          createdAt: "Created At",
          updatedAt: "Updated At",
          statusReady: "Ready",
          statusProcessing: "Processing",
          statusFailed: "Failed",
        };

  // Close modal on Escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && selectedReviewId) {
        setSelectedReviewId(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedReviewId]);

  // Queries (only enabled for investors)
  const summaryQuery = useQuery({
    queryKey: scopedKey(context, "review", "summary"),
    queryFn: () => host.call<GetSummaryResponse>("get_summary"),
    enabled: isInvestor,
  });

  const criteriaQuery = useQuery({
    queryKey: scopedKey(context, "review", "criteria"),
    queryFn: () => host.call<ListCriteriaResponse>("list_criteria"),
    enabled: isInvestor,
  });

  const reviewsQuery = useQuery({
    queryKey: scopedKey(context, "review", "list"),
    queryFn: () => host.call<ListReviewsResponse>("list_reviews"),
    enabled: isInvestor,
  });

  const detailQuery = useQuery({
    queryKey: scopedKey(context, "review", "detail", selectedReviewId),
    queryFn: () =>
      selectedReviewId
        ? host.call<GetReviewResponse>("get_review", { reviewId: selectedReviewId })
        : null,
    enabled: isInvestor && !!selectedReviewId,
  });

  const renderReviewStatusBadge = (status: ReviewStatus) => {
    switch (status) {
      case "satisfied":
        return <Badge variant="success">{t.satisfied}</Badge>;
      case "needs_information":
        return <Badge variant="warning">{t.needsInformation}</Badge>;
    }
  };

  const renderMaterialStatusBadge = (status: string) => {
    switch (status) {
      case "ready":
        return <Badge variant="success">{t.statusReady}</Badge>;
      case "processing":
        return <Badge variant="warning">{t.statusProcessing}</Badge>;
      case "failed":
        return <Badge variant="danger">{t.statusFailed}</Badge>;
      default:
        return <Badge variant="default">{status}</Badge>;
    }
  };

  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return new Intl.DateTimeFormat(context.locale === "ko" ? "ko-KR" : "en-US", {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(date);
    } catch {
      return dateStr;
    }
  };

  // Company Role Guard
  if (!isInvestor) {
    return (
      <section className="space-y-6">
        <div>
          <h1 className="text-heading-4 font-semibold">{t.pluginTitle}</h1>
          <p className="text-caption text-muted-foreground">{t.pluginSubtitle}</p>
        </div>
        <Card className="border-warning/30 bg-warning-muted p-6">
          <div role="status" className="space-y-2">
            <h2 className="text-heading-5 font-semibold text-warning">
              {t.companyRestrictedTitle}
            </h2>
            <p className="text-body-sm text-muted-foreground">{t.companyRestrictedDesc}</p>
          </div>
        </Card>
      </section>
    );
  }

  const isInitialLoading =
    summaryQuery.isPending || criteriaQuery.isPending || reviewsQuery.isPending;
  const isInitialError = summaryQuery.isError || criteriaQuery.isError || reviewsQuery.isError;

  const handleRetryAll = () => {
    void summaryQuery.refetch();
    void criteriaQuery.refetch();
    void reviewsQuery.refetch();
  };

  const summary = summaryQuery.data?.summary;
  const criteria: ReviewCriterion[] = criteriaQuery.data?.criteria ?? [];
  const reviews: ReviewSummaryItem[] = reviewsQuery.data?.reviews ?? [];

  // Map review status by criterionId for quick lookup
  const reviewMapByCriterion = new Map<string, ReviewSummaryItem>();
  for (const review of reviews) {
    reviewMapByCriterion.set(review.criterionId, review);
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-heading-4 font-semibold">{t.pluginTitle}</h1>
        <p className="text-caption text-muted-foreground">{t.pluginSubtitle}</p>
      </div>

      {/* Loading state */}
      {isInitialLoading ? (
        <Card className="p-8 text-center text-muted-foreground">
          <p role="status">{t.loading}</p>
        </Card>
      ) : isInitialError ? (
        /* Error state */
        <Card className="border-destructive/30 bg-destructive-muted p-6">
          <div role="alert" className="space-y-4">
            <p className="text-destructive font-medium">{t.fetchError}</p>
            <Button variant="outline" onClick={handleRetryAll}>
              {t.retry}
            </Button>
          </div>
        </Card>
      ) : (
        <>
          {/* Progress Summary Section */}
          <section aria-labelledby="progress-summary-title" className="space-y-4">
            <h2 id="progress-summary-title" className="text-heading-5 font-semibold">
              {t.progressSummary}
            </h2>
            {summary ? (
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
                <Card className="p-4">
                  <p className="text-caption text-muted-foreground">{t.completed}</p>
                  <p className="text-heading-3 font-bold text-foreground">{summary.completed}</p>
                </Card>
                <Card className="p-4">
                  <p className="text-caption text-muted-foreground">{t.remaining}</p>
                  <p className="text-heading-3 font-bold text-foreground">{summary.remaining}</p>
                </Card>
                <Card className="p-4">
                  <p className="text-caption text-muted-foreground">{t.satisfied}</p>
                  <p className="text-heading-3 font-bold text-success">{summary.satisfied}</p>
                </Card>
                <Card className="p-4">
                  <p className="text-caption text-muted-foreground">{t.needsInformation}</p>
                  <p className="text-heading-3 font-bold text-warning">
                    {summary.needsInformation}
                  </p>
                </Card>
              </div>
            ) : null}
          </section>

          {/* Criteria Section */}
          <section aria-labelledby="criteria-section-title" className="space-y-4">
            <h2 id="criteria-section-title" className="text-heading-5 font-semibold">
              {t.criteriaTitle}
            </h2>
            <div className="grid gap-4 md:grid-cols-3">
              {criteria.map((criterion) => {
                const existingReview = reviewMapByCriterion.get(criterion.id);
                return (
                  <Card key={criterion.id} className="flex flex-col justify-between">
                    <CardHeader className="space-y-2">
                      <div className="flex items-start justify-between gap-2">
                        <CardTitle className="text-heading-5 font-semibold">
                          {criterion.title}
                        </CardTitle>
                        {existingReview ? (
                          renderReviewStatusBadge(existingReview.status)
                        ) : (
                          <Badge variant="default">{t.pendingReview}</Badge>
                        )}
                      </div>
                      <p className="text-body-sm text-muted-foreground">
                        {criterion.reviewQuestion}
                      </p>
                    </CardHeader>
                    {existingReview ? (
                      <CardContent className="pt-0">
                        <Button
                          variant="outline"
                          size="sm"
                          className="w-full"
                          onClick={() => setSelectedReviewId(existingReview.id)}
                        >
                          {t.viewDetails}
                        </Button>
                      </CardContent>
                    ) : null}
                  </Card>
                );
              })}
            </div>
          </section>

          {/* Submitted Reviews Section */}
          <section aria-labelledby="reviews-section-title" className="space-y-4">
            <h2 id="reviews-section-title" className="text-heading-5 font-semibold">
              {t.reviewsTitle}
            </h2>
            {reviews.length === 0 ? (
              <Card className="p-8 text-center text-muted-foreground">
                <p className="font-medium">{t.noReviewsYet}</p>
                <p className="mt-1 text-caption">{t.newInvestorHelp}</p>
              </Card>
            ) : (
              <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                {reviews.map((review) => (
                  <Card
                    key={review.id}
                    className="flex flex-col justify-between transition-shadow hover:shadow-md"
                  >
                    <CardHeader className="space-y-2">
                      <div className="flex items-start justify-between gap-2">
                        <CardTitle className="text-heading-5 font-semibold">
                          {review.criterionTitle}
                        </CardTitle>
                        {renderReviewStatusBadge(review.status)}
                      </div>
                      <p className="text-caption text-muted-foreground">
                        {t.updatedAt}: {formatDate(review.updatedAt)}
                      </p>
                    </CardHeader>
                    <CardContent className="space-y-4 pt-0">
                      <p className="text-body-sm text-muted-foreground line-clamp-3">
                        {review.opinion}
                      </p>
                      <Button
                        variant="outline"
                        size="sm"
                        className="w-full"
                        onClick={() => setSelectedReviewId(review.id)}
                      >
                        {t.viewDetails}
                      </Button>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </section>
        </>
      )}

      {/* Review Detail Modal */}
      {selectedReviewId ? (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="review-detail-title"
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
          onClick={(e) => {
            if (e.target === e.currentTarget) {
              setSelectedReviewId(null);
            }
          }}
        >
          <div className="flex max-h-[90vh] w-full max-w-2xl flex-col rounded-lg border border-border bg-card shadow-lg">
            <div className="flex items-center justify-between border-b border-border p-6">
              <h2 id="review-detail-title" className="text-heading-5 font-semibold">
                {detailQuery.data?.review.criterionTitle ?? t.reviewDetail}
              </h2>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setSelectedReviewId(null)}
                aria-label={t.close}
              >
                ✕
              </Button>
            </div>

            <div className="flex-1 overflow-y-auto p-6 space-y-6">
              {detailQuery.isPending ? (
                <p role="status" className="p-6 text-center text-muted-foreground">
                  {t.detailLoading}
                </p>
              ) : detailQuery.isError ? (
                <div role="alert" className="space-y-4 rounded-lg bg-destructive-muted p-4">
                  <p className="text-destructive font-medium">{t.fetchDetailError}</p>
                  <Button variant="outline" onClick={() => detailQuery.refetch()}>
                    {t.retry}
                  </Button>
                </div>
              ) : detailQuery.data?.review ? (
                <>
                  {/* Meta Bar */}
                  <div className="flex flex-wrap items-center gap-4 rounded-md bg-secondary/50 p-3 text-caption">
                    <div>
                      <span className="font-semibold">{t.status}:</span>{" "}
                      {renderReviewStatusBadge(detailQuery.data.review.status)}
                    </div>
                    <div>
                      <span className="font-semibold">{t.createdAt}:</span>{" "}
                      <span>{formatDate(detailQuery.data.review.createdAt)}</span>
                    </div>
                    <div>
                      <span className="font-semibold">{t.updatedAt}:</span>{" "}
                      <span>{formatDate(detailQuery.data.review.updatedAt)}</span>
                    </div>
                  </div>

                  {/* Review Question */}
                  <div className="space-y-1">
                    <h3 className="text-caption font-semibold text-muted-foreground uppercase tracking-wider">
                      {t.reviewQuestion}
                    </h3>
                    <p className="text-body-sm font-medium">
                      {detailQuery.data.review.reviewQuestion}
                    </p>
                  </div>

                  {/* Opinion */}
                  <div className="space-y-2">
                    <h3 className="text-caption font-semibold text-muted-foreground uppercase tracking-wider">
                      {t.opinion}
                    </h3>
                    <div className="rounded-md border border-border bg-muted/20 p-4 text-body-sm whitespace-pre-wrap break-words">
                      {detailQuery.data.review.opinion}
                    </div>
                  </div>

                  {/* Evidence Materials */}
                  <div className="space-y-3">
                    <h3 className="text-caption font-semibold text-muted-foreground uppercase tracking-wider">
                      {t.evidence} ({detailQuery.data.review.evidence.length})
                    </h3>
                    {detailQuery.data.review.evidence.length === 0 ? (
                      <p className="text-caption text-muted-foreground">{t.noEvidence}</p>
                    ) : (
                      <div className="divide-y divide-border rounded-md border border-border">
                        {detailQuery.data.review.evidence.map((item) => (
                          <div key={item.id} className="flex items-center justify-between p-3">
                            <div className="space-y-0.5">
                              <p className="text-body-sm font-medium">{item.title}</p>
                              <p className="text-caption text-muted-foreground">{item.fileName}</p>
                            </div>
                            <div>{renderMaterialStatusBadge(item.status)}</div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </>
              ) : null}
            </div>

            <div className="flex justify-end border-t border-border p-4">
              <Button variant="outline" onClick={() => setSelectedReviewId(null)}>
                {t.close}
              </Button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
};
