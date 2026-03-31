import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TagList } from "./TagList";

describe("TagList", () => {
  it("renders tags", () => {
    render(<TagList label="Items" items={["orchid", "bloodthorn"]} onChange={() => {}} />);
    expect(screen.getByText("orchid")).toBeInTheDocument();
    expect(screen.getByText("bloodthorn")).toBeInTheDocument();
  });

  it("removes a tag when × is clicked", async () => {
    const onChange = vi.fn();
    render(<TagList label="Items" items={["orchid", "bloodthorn"]} onChange={onChange} />);
    const removeButtons = screen.getAllByRole("button", { name: /remove/i });
    await userEvent.click(removeButtons[0]);
    expect(onChange).toHaveBeenCalledWith(["bloodthorn"]);
  });

  it("adds a tag via input", async () => {
    const onChange = vi.fn();
    render(<TagList label="Items" items={["orchid"]} onChange={onChange} />);
    const addBtn = screen.getByRole("button", { name: /add/i });
    await userEvent.click(addBtn);
    const input = screen.getByPlaceholderText("Add item...");
    await userEvent.type(input, "nullifier{enter}");
    expect(onChange).toHaveBeenCalledWith(["orchid", "nullifier"]);
  });
});
