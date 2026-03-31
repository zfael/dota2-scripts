import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Sidebar } from "./Sidebar";

function renderSidebar() {
  return render(
    <MemoryRouter initialEntries={["/"]}>
      <Sidebar />
    </MemoryRouter>,
  );
}

describe("Sidebar", () => {
  it("renders all navigation items", () => {
    renderSidebar();
    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Heroes")).toBeInTheDocument();
    expect(screen.getByText("Danger")).toBeInTheDocument();
    expect(screen.getByText("Soul Ring")).toBeInTheDocument();
    expect(screen.getByText("Armlet")).toBeInTheDocument();
    expect(screen.getByText("Activity")).toBeInTheDocument();
    expect(screen.getByText("Diagnostics")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("renders version in footer", () => {
    renderSidebar();
    expect(screen.getByText(/v\d/)).toBeInTheDocument();
  });
});
