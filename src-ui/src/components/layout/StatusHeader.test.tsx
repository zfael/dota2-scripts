import { render, screen } from "@testing-library/react";
import { StatusHeader } from "./StatusHeader";

const defaultProps = {
  stunned: false,
  silenced: false,
  alive: true,
  respawnTimer: null,
};

describe("StatusHeader", () => {
  it("renders idle state when no game data", () => {
    render(<StatusHeader {...defaultProps} />);
    expect(screen.getByText("Waiting for game...")).toBeInTheDocument();
  });

  it("renders in-game state with hero info", () => {
    render(
      <StatusHeader
        heroName="Shadow Fiend"
        heroLevel={15}
        hpPercent={72}
        manaPercent={55}
        inDanger={false}
        connected={true}
        {...defaultProps}
      />,
    );
    expect(screen.getByText("Shadow Fiend")).toBeInTheDocument();
    expect(screen.getByText("Lv. 15")).toBeInTheDocument();
    expect(screen.getByText("72%")).toBeInTheDocument();
    expect(screen.getByText("55%")).toBeInTheDocument();
  });

  it("shows danger badge when in danger", () => {
    render(
      <StatusHeader
        heroName="Huskar"
        heroLevel={10}
        hpPercent={20}
        manaPercent={40}
        inDanger={true}
        connected={true}
        {...defaultProps}
      />,
    );
    expect(screen.getByText("⚠ DANGER")).toBeInTheDocument();
  });
});
