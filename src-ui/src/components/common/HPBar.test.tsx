import { render, screen } from "@testing-library/react";
import { HPBar } from "./HPBar";
import { ManaBar } from "./ManaBar";

describe("HPBar", () => {
  it("renders percentage text", () => {
    render(<HPBar percent={75} />);
    expect(screen.getByText("75%")).toBeInTheDocument();
  });

  it("applies green color at high HP", () => {
    const { container } = render(<HPBar percent={80} />);
    const fill = container.querySelector("[data-fill]");
    expect(fill).toHaveStyle({ width: "80%" });
  });

  it("applies danger color at low HP", () => {
    const { container } = render(<HPBar percent={20} />);
    const fill = container.querySelector("[data-fill]");
    expect(fill?.className).toContain("bg-danger");
  });
});

describe("ManaBar", () => {
  it("renders percentage text", () => {
    render(<ManaBar percent={60} />);
    expect(screen.getByText("60%")).toBeInTheDocument();
  });
});
