import { test, expect, type Page } from "@playwright/test";
import { loginAs, logout } from "./support/auth";

function getProgressCard(page: Page, labelRegex: RegExp) {
  const progressSection = page
    .locator("section")
    .filter({ hasText: /검토 진행 현황|Review Progress/i });
  return progressSection.locator(".grid > div").filter({ hasText: labelRegex });
}

test.describe("Review Plugin", () => {
  test("Scenario 3: Company restriction - reviews are private to investors", async ({
    page,
  }) => {
    // 1. Login as company user
    await loginAs(page, "company@lighthouse.test");

    // 2. Open Review Plugin
    await page.getByRole("link", { name: /Review Plugin|검토/i }).click();

    // 3. The investor-only restriction message is visible
    await expect(
      page.getByRole("heading", {
        name: /투자자 전용 기능|Investor-Only Feature/i,
      }),
    ).toBeVisible();

    // 4. Investor progress is not visible
    await expect(
      page.getByText(/검토 진행 현황|Review Progress/i),
    ).not.toBeVisible();

    // 5. Submitted investor reviews are not visible
    await expect(
      page.getByText(/작성된 검토 목록|Submitted Reviews/i),
    ).not.toBeVisible();

    // 6. Review creation controls are not visible
    await expect(
      page.getByRole("button", { name: /새 검토 작성|Write Review/i }),
    ).not.toBeVisible();

    // 7. Review editing controls are not visible
    await expect(
      page.getByRole("button", { name: /수정|Edit/i }),
    ).not.toBeVisible();

    // 8. Evidence selection is not exposed
    await expect(
      page.getByText(/증빙 자료 선택|Evidence Materials/i),
    ).not.toBeVisible();
  });

  test("Scenario 4 & 5: Investor creation, evidence selection, and editing with needs_information", async ({
    page,
  }) => {
    // 1. Login as investor user
    await loginAs(page, "investor@lighthouse.test");

    // 2. DataRoom registration is unavailable for investor
    await page.getByRole("link", { name: /데이터룸|Data room/i }).click();
    await expect(
      page.getByRole("button", { name: /자료 등록|Register material/i }),
    ).not.toBeVisible();

    // 3. Review Plugin loads
    await page.getByRole("link", { name: /Review Plugin|검토/i }).click();
    await expect(
      page.getByRole("heading", { name: /투자 검토 대시보드|Review Dashboard/i }),
    ).toBeVisible();

    // 4. Verify progress cards are rendered
    const completedCard = getProgressCard(page, /완료됨|Completed/i);
    const remainingCard = getProgressCard(page, /남음|Remaining/i);
    const satisfiedCard = getProgressCard(page, /충족|Satisfied/i);
    const needsInfoCard = getProgressCard(page, /정보 필요|Needs Information/i);

    await expect(completedCard).toBeVisible();
    await expect(remainingCard).toBeVisible();

    // 5. Verify the three fixed criteria exist in the Criteria section
    const criteriaSection = page
      .locator("section")
      .filter({ hasText: /검토 기준|Review Criteria/i });
    await expect(criteriaSection.getByText(/사업 이해|Business/i)).toBeVisible();
    await expect(criteriaSection.getByText(/팀 구성|Team/i)).toBeVisible();
    await expect(criteriaSection.getByText(/매출 현황|Revenue/i)).toBeVisible();

    // 6. Open editor for business criterion
    const businessCriterionCard = criteriaSection
      .locator(".grid > div")
      .filter({ hasText: /사업 이해|Business/i });
    await businessCriterionCard
      .getByRole("button", { name: /검토 작성|수정|Review|Edit/i })
      .click();

    const editorModal = page.getByRole("dialog");
    await expect(editorModal).toBeVisible();

    // 7. Select status = satisfied
    await editorModal.locator("#review-status").selectOption("satisfied");

    // 8. Enter a valid opinion
    const initialOpinion = "Strong business model with recurring revenue streams.";
    await editorModal.locator("#review-opinion").fill(initialOpinion);

    // 9. Verify evidence selection options
    const companyOverviewCheckbox = editorModal
      .locator("label")
      .filter({ hasText: "company-overview.md" })
      .locator('input[type="checkbox"]');
    const teamCheckbox = editorModal
      .locator("label")
      .filter({ hasText: "team.md" })
      .locator('input[type="checkbox"]');
    const revenueCheckbox = editorModal
      .locator(".divide-y > div")
      .filter({ hasText: "revenue.txt" })
      .locator('input[type="checkbox"]');
    const customerInterviewsCheckbox = editorModal
      .locator(".divide-y > div")
      .filter({ hasText: "customer-interviews.md" })
      .locator('input[type="checkbox"]');

    await expect(companyOverviewCheckbox).toBeEnabled();
    await expect(teamCheckbox).toBeEnabled();
    await expect(revenueCheckbox).toBeDisabled();
    await expect(customerInterviewsCheckbox).toBeDisabled();

    // 10. Select company-overview.md as the only evidence
    if (!(await companyOverviewCheckbox.isChecked())) {
      await companyOverviewCheckbox.check();
    }
    if (await teamCheckbox.isChecked()) {
      await teamCheckbox.uncheck();
    }

    // 11. Save the review
    await editorModal
      .getByRole("button", { name: /저장|Save/i, exact: true })
      .click();
    await expect(editorModal).not.toBeVisible();

    // Verify progress: completed=1, remaining=2, satisfied=1, needs_info=0
    await expect(completedCard).toContainText("1");
    await expect(remainingCard).toContainText("2");
    await expect(satisfiedCard).toContainText("1");
    await expect(needsInfoCard).toContainText("0");

    // Verify business review card exists in submitted reviews
    const reviewsSection = page
      .locator("section")
      .filter({ hasText: /작성된 검토 목록|Submitted Reviews/i });
    const businessReviewCard = reviewsSection
      .locator(".grid > div")
      .filter({ hasText: /사업 이해/i });
    await expect(businessReviewCard).toBeVisible();
    await expect(businessReviewCard.getByText(/충족|Satisfied/i)).toBeVisible();

    // ----------------------------------------------------
    // SCENARIO 5: Edit review and change to needs_information
    // ----------------------------------------------------

    // 1. Open business review detail modal
    await businessReviewCard
      .getByRole("button", { name: /상세 보기|View Details/i })
      .click();
    const detailModal = page.getByRole("dialog");
    await expect(detailModal).toBeVisible();

    // 2. Verify detail content
    await expect(detailModal.getByText(/충족|Satisfied/i)).toBeVisible();
    await expect(detailModal).toContainText(initialOpinion);
    await expect(detailModal.getByText("company-overview.md")).toBeVisible();

    // 3. Enter edit mode
    await detailModal
      .getByRole("button", { name: /검토 수정|Edit Review/i })
      .click();

    // 4. Verify criterion is read-only in edit mode
    await expect(editorModal).toBeVisible();
    await expect(editorModal.locator("#review-criterion")).not.toBeVisible();

    // 5. Change status to needs_information
    await editorModal.locator("#review-status").selectOption("needs_information");

    // 6. Change opinion
    const updatedOpinion =
      "Need additional clarification on churn rate and unit economics.";
    await editorModal.locator("#review-opinion").fill(updatedOpinion);

    // 7. Add team.md as evidence (both company-overview.md and team.md checked)
    const editTeamCheckbox = editorModal
      .locator("label")
      .filter({ hasText: "team.md" })
      .locator('input[type="checkbox"]');
    if (!(await editTeamCheckbox.isChecked())) {
      await editTeamCheckbox.check();
    }
    await expect(editTeamCheckbox).toBeChecked();

    // 8. Save edited review
    await editorModal
      .getByRole("button", { name: /저장|Save/i, exact: true })
      .click();
    await expect(editorModal).not.toBeVisible();

    // Verify progress: completed=1, remaining=2, satisfied=0, needs_info=1
    await expect(completedCard).toContainText("1");
    await expect(remainingCard).toContainText("2");
    await expect(satisfiedCard).toContainText("0");
    await expect(needsInfoCard).toContainText("1");

    // Verify review card reflects needs_information and updated opinion
    await expect(
      businessReviewCard.getByText(/정보 필요|Needs Information/i),
    ).toBeVisible();
    await expect(businessReviewCard).toContainText(updatedOpinion);

    // Open detail modal to verify both evidence materials
    await businessReviewCard
      .getByRole("button", { name: /상세 보기|View Details/i })
      .click();
    await expect(detailModal).toBeVisible();
    await expect(detailModal.getByText("company-overview.md")).toBeVisible();
    await expect(detailModal.getByText("team.md")).toBeVisible();
    await detailModal
      .getByRole("button", { name: /닫기|Close/i })
      .last()
      .click();
    await expect(detailModal).not.toBeVisible();
  });

  test("Scenario 6: Multi-investor isolation - peer has independent reviews and progress", async ({
    page,
  }) => {
    // 1. Login as peer investor
    await loginAs(page, "peer@lighthouse.test");

    // 2. Open Review Plugin
    await page.getByRole("link", { name: /Review Plugin|검토/i }).click();
    await expect(
      page.getByRole("heading", { name: /투자 검토 대시보드|Review Dashboard/i }),
    ).toBeVisible();

    // 3. Verify peer cannot see investor's business review
    const reviewsSection = page
      .locator("section")
      .filter({ hasText: /작성된 검토 목록|Submitted Reviews/i });
    await expect(
      reviewsSection.locator(".grid > div").filter({ hasText: /사업 이해/i }),
    ).not.toBeVisible();

    // 4. Peer creates/updates a review for criterion = team, status = satisfied
    const criteriaSection = page
      .locator("section")
      .filter({ hasText: /검토 기준|Review Criteria/i });
    const teamCriterionCard = criteriaSection
      .locator(".grid > div")
      .filter({ hasText: /팀 구성|Team/i });
    await teamCriterionCard
      .getByRole("button", { name: /검토 작성|수정|Review|Edit/i })
      .click();

    const editorModal = page.getByRole("dialog");
    await expect(editorModal).toBeVisible();
    await editorModal.locator("#review-status").selectOption("satisfied");
    await editorModal
      .locator("#review-opinion")
      .fill("Experienced core team with strong domain expertise.");

    const teamCheckbox = editorModal
      .locator("label")
      .filter({ hasText: "team.md" })
      .locator('input[type="checkbox"]');
    if (!(await teamCheckbox.isChecked())) {
      await teamCheckbox.check();
    }

    // 5. Save peer's review
    await editorModal
      .getByRole("button", { name: /저장|Save/i, exact: true })
      .click();
    await expect(editorModal).not.toBeVisible();

    // 6. Verify peer's progress becomes 1 completed / 2 remaining
    const completedCard = getProgressCard(page, /완료됨|Completed/i);
    const remainingCard = getProgressCard(page, /남음|Remaining/i);
    await expect(completedCard).toContainText("1");
    await expect(remainingCard).toContainText("2");

    // 7. Logout peer
    await logout(page);

    // 8. Login again as investor@lighthouse.test
    await loginAs(page, "investor@lighthouse.test");
    await page.getByRole("link", { name: /Review Plugin|검토/i }).click();
    await expect(
      page.getByRole("heading", { name: /투자 검토 대시보드|Review Dashboard/i }),
    ).toBeVisible();

    // 9. Verify investor's business review still exists with needs_information
    const investorReviewsSection = page
      .locator("section")
      .filter({ hasText: /작성된 검토 목록|Submitted Reviews/i });
    const businessReviewCard = investorReviewsSection
      .locator(".grid > div")
      .filter({ hasText: /사업 이해/i });
    await expect(businessReviewCard).toBeVisible();
    await expect(
      businessReviewCard.getByText(/정보 필요|Needs Information/i),
    ).toBeVisible();

    // 10. Verify investor does not see peer's team review
    await expect(
      investorReviewsSection.locator(".grid > div").filter({ hasText: /팀 구성/i }),
    ).not.toBeVisible();
  });

  test("Scenario 7: Review form validation and boundaries", async ({ page }) => {
    // 1. Login as investor user
    await loginAs(page, "investor@lighthouse.test");
    await page.getByRole("link", { name: /Review Plugin|검토/i }).click();

    // 2. Open review editor for revenue (or open edit)
    const criteriaSection = page
      .locator("section")
      .filter({ hasText: /검토 기준|Review Criteria/i });
    const revenueCard = criteriaSection
      .locator(".grid > div")
      .filter({ hasText: /매출 현황|Revenue/i });

    const reviewButton = revenueCard.getByRole("button", {
      name: /검토 작성|수정|Review|Edit/i,
    });
    await reviewButton.click();

    const editorModal = page.getByRole("dialog");
    await expect(editorModal).toBeVisible();

    const saveButton = editorModal.getByRole("button", {
      name: /저장|Save/i,
      exact: true,
    });
    const opinionTextarea = editorModal.locator("#review-opinion");

    // 3. Verify empty opinion cannot be submitted (Save button disabled)
    await opinionTextarea.fill("");
    await expect(saveButton).toBeDisabled();

    // 4. Verify whitespace-only opinion cannot be submitted
    await opinionTextarea.fill("   \n\t  ");
    await expect(saveButton).toBeDisabled();

    // 5. Verify valid opinion with zero evidence cannot be submitted
    await opinionTextarea.fill("Valid opinion without evidence.");
    // Uncheck all evidence checkboxes if any
    const companyOverviewCheckbox = editorModal
      .locator("label")
      .filter({ hasText: "company-overview.md" })
      .locator('input[type="checkbox"]');
    const teamCheckbox = editorModal
      .locator("label")
      .filter({ hasText: "team.md" })
      .locator('input[type="checkbox"]');
    if (await companyOverviewCheckbox.isChecked()) {
      await companyOverviewCheckbox.uncheck();
    }
    if (await teamCheckbox.isChecked()) {
      await teamCheckbox.uncheck();
    }
    await expect(saveButton).toBeDisabled();

    // 6. Verify character counter display
    await expect(editorModal.getByText(/\/ 2000/)).toBeVisible();

    // 7. Test 2000-character opinion reaches boundary
    const maxOpinion = "A".repeat(2000);
    await opinionTextarea.fill(maxOpinion);
    await expect(editorModal.getByText("2000 / 2000")).toBeVisible();

    // Check an evidence material to verify save button enables at 2000 chars
    await companyOverviewCheckbox.check();
    await expect(saveButton).toBeEnabled();

    // Test 2001 characters disables save button
    await opinionTextarea.fill("A".repeat(2001));
    await expect(editorModal.getByText("2001 / 2000")).toBeVisible();
    await expect(saveButton).toBeDisabled();

    // 8. Cancel form and verify clean dismissal
    await editorModal.getByRole("button", { name: /취소|Cancel/i }).click();
    await expect(editorModal).not.toBeVisible();
  });
});
