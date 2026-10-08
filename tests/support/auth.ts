import { expect, type Page } from "@playwright/test";

export async function logout(page: Page): Promise<void> {
  const logoutButton = page.getByRole("button", { name: /로그아웃|Sign out/i });
  if (await logoutButton.isVisible()) {
    await logoutButton.click();
    await expect(page.getByRole("button", { name: /로그인|Sign in/i, exact: true })).toBeVisible();
  }
}

export async function loginAs(
  page: Page,
  email: string,
  password = "dataroom",
): Promise<void> {
  await page.goto("/");

  const logoutButton = page.getByRole("button", { name: /로그아웃|Sign out/i });
  const emailInput = page.getByLabel(/이메일|Email/i);
  const submitButton = page.getByRole("button", { name: /로그인|Sign in/i, exact: true });

  // Wait for either authenticated shell or login form to be visible
  await Promise.race([
    logoutButton.waitFor({ state: "visible" }).catch(() => {}),
    emailInput.waitFor({ state: "visible" }).catch(() => {}),
  ]);

  if (await logoutButton.isVisible()) {
    await logoutButton.click();
    await expect(submitButton).toBeVisible();
  }

  await expect(emailInput).toBeVisible();
  await emailInput.fill(email);
  await page.getByLabel(/비밀번호|Password/i).fill(password);
  await submitButton.click();

  // Wait for authenticated shell workspace UI
  await expect(logoutButton).toBeVisible();
  await expect(page.getByRole("link", { name: /데이터룸|Data room/i })).toBeVisible();
}
