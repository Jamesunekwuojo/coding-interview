import { test, expect } from "@playwright/test";
import { loginAs } from "./support/auth";

test.describe("DataRoom", () => {
  test("Scenario 1: Company workflow - list, search, register, and view material", async ({
    page,
  }) => {
    // 1. Login as company user
    await loginAs(page, "company@lighthouse.test");

    // 2. Ensure DataRoom is loaded
    await page.getByRole("link", { name: /데이터룸|Data room/i }).click();
    const searchInput = page.getByPlaceholder(/제목으로 검색|Search by title/i);
    await expect(searchInput).toBeVisible();

    // 3. Seeded materials are visible
    await expect(page.getByText("회사 소개")).toBeVisible();
    await expect(page.getByText("팀 소개")).toBeVisible();

    // 4. Search by material title works
    await searchInput.fill("회사 소개");
    await expect(page.getByText("회사 소개")).toBeVisible();
    await expect(page.getByText("팀 소개")).not.toBeVisible();

    // 5. Search can be cleared
    await searchInput.clear();
    await expect(page.getByText("팀 소개")).toBeVisible();

    // 6. "Register Material" is visible for company
    const registerButton = page.getByRole("button", {
      name: /자료 등록|Register material/i,
    });
    await expect(registerButton).toBeVisible();

    // 7. Register a unique .md material using Playwright file upload
    await registerButton.click();
    const registerModal = page.getByRole("dialog");
    await expect(registerModal).toBeVisible();

    const uniqueTitle = `E2E Material ${Date.now()}`;
    const fileContent = `# E2E Material Header\nDeterministic uploaded content for test ${Date.now()}.`;

    await registerModal.locator("#material-title").fill(uniqueTitle);
    await registerModal.locator("#material-file").setInputFiles({
      name: "e2e-material.md",
      mimeType: "text/markdown",
      buffer: Buffer.from(fileContent, "utf-8"),
    });

    // 8. Submit registration
    const submitRegister = registerModal.getByRole("button", {
      name: /등록하기|Register/i,
      exact: true,
    });
    await submitRegister.click();

    // 9. Verify the new material appears in DataRoom
    await expect(registerModal).not.toBeVisible();
    await expect(page.getByText(uniqueTitle)).toBeVisible();

    // 10. Verify the material has Ready status
    const newMaterialCard = page
      .locator(".grid > div")
      .filter({ hasText: uniqueTitle });
    await expect(newMaterialCard.getByText(/준비 완료|Ready/i)).toBeVisible();

    // 11. Open the material content
    await newMaterialCard
      .getByRole("button", { name: /본문 보기|View content/i })
      .click();

    // 12. Verify the uploaded content in the detail modal
    const detailModal = page.getByRole("dialog");
    await expect(detailModal).toBeVisible();
    await expect(detailModal).toContainText("Deterministic uploaded content for test");

    // 13. Close the content modal
    await detailModal
      .getByRole("button", { name: /닫기|Close/i })
      .last()
      .click();
    await expect(detailModal).not.toBeVisible();
  });

  test("Scenario 2: Investor restriction - view and search without registration control", async ({
    page,
  }) => {
    // 1. Login as investor user
    await loginAs(page, "investor@lighthouse.test");

    // 2. DataRoom loads
    await page.getByRole("link", { name: /데이터룸|Data room/i }).click();
    const searchInput = page.getByPlaceholder(/제목으로 검색|Search by title/i);
    await expect(searchInput).toBeVisible();

    // 3. Materials can be viewed
    await expect(page.getByText("회사 소개")).toBeVisible();
    await expect(page.getByText("팀 소개")).toBeVisible();

    // 4. Search works
    await searchInput.fill("팀 소개");
    await expect(page.getByText("팀 소개")).toBeVisible();
    await expect(page.getByText("회사 소개")).not.toBeVisible();
    await searchInput.clear();
    await expect(page.getByText("회사 소개")).toBeVisible();

    // 5. Material detail can be opened
    const overviewCard = page
      .locator(".grid > div")
      .filter({ hasText: "회사 소개" });
    await overviewCard
      .getByRole("button", { name: /본문 보기|View content/i })
      .click();
    const detailModal = page.getByRole("dialog");
    await expect(detailModal).toBeVisible();
    await expect(detailModal).toContainText("제조사 재고 관리 구독형 소프트웨어");
    await detailModal
      .getByRole("button", { name: /닫기|Close/i })
      .last()
      .click();
    await expect(detailModal).not.toBeVisible();

    // 6. "Register Material" is NOT rendered
    await expect(
      page.getByRole("button", { name: /자료 등록|Register material/i }),
    ).not.toBeVisible();
  });
});
