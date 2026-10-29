import { describe, it, expect } from "vitest";
import { applyOverlayBodyStyles, isOverlayWindow, overlayKind } from "./overlay";

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

describe("overlayKind", () => {
  it("keeps the wave overlay's existing URL as the wave overlay", () => {
    expect(overlayKind("?overlay=1")).toBe("wave");
    expect(overlayKind("?overlay")).toBe("wave");
  });

  it("tells the latch indicator apart", () => {
    expect(overlayKind("?overlay=latch")).toBe("latch");
  });

  it("is null for the main window", () => {
    expect(overlayKind("")).toBeNull();
    expect(overlayKind("?overlayed=latch")).toBeNull();
  });
});

describe("applyOverlayBodyStyles", () => {
  it("marks the element so the stylesheet can drop the opaque background", () => {
    const element = document.createElement("div");
    applyOverlayBodyStyles(element);

    expect(element.classList.contains("overlay-mode")).toBe(true);
  });
});
