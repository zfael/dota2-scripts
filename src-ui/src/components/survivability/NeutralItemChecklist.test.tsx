import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { NeutralItemChecklist } from "./NeutralItemChecklist";
import { mockNeutralItems } from "../../stores/mockData";

describe("NeutralItemChecklist", () => {
  it("ticks the allowed items and labels the cast mode", () => {
    render(
      <NeutralItemChecklist pool={mockNeutralItems} allowed={["item_flicker"]} onChange={() => {}} />,
    );
    expect(screen.getByRole("checkbox", { name: /Flicker/ })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: /Minotaur Horn/ })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: /Minotaur Horn/ })).toHaveAccessibleName(
      /Self-cast/,
    );
  });

  it("adds a supported item to the end of the list", async () => {
    const onChange = vi.fn();
    render(
      <NeutralItemChecklist pool={mockNeutralItems} allowed={["item_flicker"]} onChange={onChange} />,
    );
    await userEvent.click(screen.getByRole("checkbox", { name: /Brick/ }));
    expect(onChange).toHaveBeenCalledWith(["item_flicker", "item_brick"]);
  });

  it("locks items the danger path cannot fire, and hides passives", () => {
    render(<NeutralItemChecklist pool={mockNeutralItems} allowed={[]} onChange={() => {}} />);
    expect(screen.getByRole("checkbox", { name: /Seeds of Serenity/ })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: /Pollywog Charm/ })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: /Tumbler's Toy/ })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: /Mana Draught/ })).toBeDisabled();
    expect(screen.queryByRole("checkbox", { name: /Chipped Vest/ })).not.toBeInTheDocument();
    expect(screen.getByText(/1 passive, nothing to cast/)).toBeInTheDocument();
  });

  it("lets a stale tick on an unsupported item be cleared", async () => {
    const onChange = vi.fn();
    render(
      <NeutralItemChecklist
        pool={mockNeutralItems}
        allowed={["item_seeds_of_serenity"]}
        onChange={onChange}
      />,
    );
    const seeds = screen.getByRole("checkbox", { name: /Seeds of Serenity/ });
    expect(seeds).toBeEnabled();
    await userEvent.click(seeds);
    expect(onChange).toHaveBeenCalledWith([]);
  });

  it("shows items that left the pool so they can be removed", async () => {
    const onChange = vi.fn();
    render(
      <NeutralItemChecklist
        pool={mockNeutralItems}
        allowed={["item_essence_ring", "item_flicker"]}
        onChange={onChange}
      />,
    );
    expect(screen.getByText("Not in the current pool")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "remove item_essence_ring" }));
    expect(onChange).toHaveBeenCalledWith(["item_flicker"]);
  });
});
