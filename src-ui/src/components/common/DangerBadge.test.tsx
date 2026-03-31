import { render, screen } from "@testing-library/react";
import { DangerBadge } from "./DangerBadge";

describe("DangerBadge", () => {
  it("renders danger text", () => {
    render(<DangerBadge />);
    expect(screen.getByText("⚠ DANGER")).toBeInTheDocument();
  });

  it("renders custom text", () => {
    render(<DangerBadge text="CRITICAL" />);
    expect(screen.getByText("CRITICAL")).toBeInTheDocument();
  });
});
