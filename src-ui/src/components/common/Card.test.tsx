import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Card } from "./Card";

describe("Card", () => {
  it("renders title and children", () => {
    render(<Card title="Settings"><p>Content here</p></Card>);
    expect(screen.getByText("Settings")).toBeInTheDocument();
    expect(screen.getByText("Content here")).toBeInTheDocument();
  });

  it("collapses content when collapsible header is clicked", async () => {
    render(
      <Card title="Collapsible" collapsible>
        <p>Hidden content</p>
      </Card>,
    );
    expect(screen.getByText("Hidden content")).toBeVisible();
    await userEvent.click(screen.getByText("Collapsible"));
    expect(screen.queryByText("Hidden content")).not.toBeVisible();
  });

  it("starts collapsed when defaultOpen is false", () => {
    render(
      <Card title="Closed" collapsible defaultOpen={false}>
        <p>Invisible</p>
      </Card>,
    );
    expect(screen.queryByText("Invisible")).not.toBeVisible();
  });
});
