import { render, screen } from "@testing-library/react";
import { NumberInput } from "./NumberInput";

describe("NumberInput", () => {
  it("renders with label and value", () => {
    render(<NumberInput label="Port" value={3000} onChange={() => {}} />);
    expect(screen.getByText("Port")).toBeInTheDocument();
    expect(screen.getByDisplayValue("3000")).toBeInTheDocument();
  });

  it("renders suffix", () => {
    render(<NumberInput label="Delay" value={100} onChange={() => {}} suffix="ms" />);
    expect(screen.getByText("ms")).toBeInTheDocument();
  });
});
