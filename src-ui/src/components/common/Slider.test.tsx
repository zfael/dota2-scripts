import { render, screen } from "@testing-library/react";
import { Slider } from "./Slider";

describe("Slider", () => {
  it("renders with label and value", () => {
    render(
      <Slider label="HP Threshold" value={70} min={0} max={100} onChange={() => {}} suffix="%" />,
    );
    expect(screen.getByText("HP Threshold")).toBeInTheDocument();
    expect(screen.getByText("70%")).toBeInTheDocument();
  });

  it("renders the range input", () => {
    render(<Slider label="Test" value={50} min={0} max={100} onChange={() => {}} />);
    const input = screen.getByRole("slider");
    expect(input).toHaveValue("50");
  });

  it("applies aria attributes", () => {
    render(
      <Slider label="Volume" value={30} min={0} max={100} onChange={() => {}} suffix="%" />,
    );
    const input = screen.getByRole("slider");
    expect(input).toHaveAttribute("aria-valuemin", "0");
    expect(input).toHaveAttribute("aria-valuemax", "100");
    expect(input).toHaveAttribute("aria-valuenow", "30");
    expect(input).toHaveAttribute("aria-valuetext", "30%");
  });
});
