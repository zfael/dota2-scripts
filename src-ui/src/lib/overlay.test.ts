import { describe, it, expect } from "vitest";
import { applyOverlayBodyStyles, isOverlayWindow } from "./overlay";

describe("isOverlayWindow", () => {
  it("detects the overlay query parameter", () => {
    expect(isOverlayWindow("?overlay=1")).toBe(true);
    expect(isOverlayWindow("?overlay")).toBe(true);
  });

  it("treats the main window as not an overlay", () => {
    expect(isOverlayWindow("")).toBe(false);
    expect(isOverlayWindow("?foo=bar")).toBe(false);
  });

  it("does not match a merely similar parameter name", () => {
    expect(isOverlayWindow("?overlayed=1")).toBe(false);
  });
});

describe("applyOverlayBodyStyles", () => {
  it("marks the element so the stylesheet can drop the opaque background", () => {
    const element = document.createElement("div");
    applyOverlayBodyStyles(element);

    expect(element.classList.contains("overlay-mode")).toBe(true);
  });
});
