import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriMocks = vi.hoisted(() => ({
  getManualCookies: vi.fn(),
  removeManualCookie: vi.fn(),
  setManualCookie: vi.fn(),
}));

vi.mock("../../../lib/tauri", () => tauriMocks);
vi.mock("../../../hooks/useLocale", () => ({
  useLocale: () => ({ t: (key: string) => key }),
}));

import { CookieSection } from "./CookieSection";

function renderSection() {
  return render(
    <CookieSection providerId="codex" cookieDomain="example.com" />,
  );
}

describe("CookieSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    tauriMocks.getManualCookies.mockResolvedValue([]);
  });

  it("saves a pasted Cookie header", async () => {
    tauriMocks.setManualCookie.mockResolvedValue([]);
    renderSection();

    const input = await screen.findByPlaceholderText("BrowserCookiePlaceholderDefault");
    expect(screen.queryByText("BrowserCookieImportFromBrowser")).not.toBeInTheDocument();

    fireEvent.change(input, { target: { value: "session=example" } });
    fireEvent.click(screen.getByRole("button", { name: "BrowserCookieSave" }));
    await waitFor(() =>
      expect(tauriMocks.setManualCookie).toHaveBeenCalledWith("codex", "session=example"),
    );
  });
});
