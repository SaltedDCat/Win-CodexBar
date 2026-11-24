import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriMocks = vi.hoisted(() => ({
  setProviderCookieSource: vi.fn(),
}));

vi.mock("../../../../lib/tauri", () => tauriMocks);

import { loadStyles, ruleBlock } from "../../../../test/styles";
import { CookieSourceSection } from "./CookieSourceSection";

const OPTIONS = [
  { value: "manual", label: "Manual" },
  { value: "off", label: "Disabled" },
];

function renderSection(
  overrides: Partial<Parameters<typeof CookieSourceSection>[0]> = {},
) {
  const onChanged = vi.fn();
  render(
    <CookieSourceSection
      providerId="ollama"
      currentValue="manual"
      options={OPTIONS}
      t={(key) => key}
      onChanged={onChanged}
      {...overrides}
    />,
  );
  return { onChanged };
}

describe("CookieSourceSection", () => {
  beforeEach(() => {
    tauriMocks.setProviderCookieSource.mockReset();
  });

  it("switches from Manual to Disabled", async () => {
    tauriMocks.setProviderCookieSource.mockResolvedValue(undefined);
    const { onChanged } = renderSection();

    fireEvent.click(screen.getByRole("radio", { name: "Disabled" }));

    await waitFor(() => expect(onChanged).toHaveBeenCalledTimes(1));
    expect(tauriMocks.setProviderCookieSource).toHaveBeenCalledWith("ollama", "off");
  });

  it("renders the picker for a cookie provider and nothing without options", () => {
    const { unmount } = render(
      <CookieSourceSection
        providerId="claude"
        currentValue="manual"
        options={OPTIONS}
        t={(key) => key}
        onChanged={vi.fn()}
      />,
    );
    expect(screen.getByRole("radiogroup", { name: "ProviderCookieSource" })).toBeInTheDocument();
    unmount();

    const { container } = render(
      <CookieSourceSection
        providerId="vercel"
        currentValue={null}
        options={[]}
        t={(key) => key}
        onChanged={vi.fn()}
      />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("wraps the segmented options inside the narrow detail pane", () => {
    const rule = ruleBlock(loadStyles(), ".provider-detail-segmented");
    expect(rule).toMatch(/flex-wrap:\s*wrap;/);
    expect(rule).toMatch(/max-width:\s*100%;/);
  });
});
