import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { KeyInput } from "./KeyInput";

/**
 * These pin the names this component emits, because the Rust side parses them.
 * A mismatch here does not throw — the binding lands in `KeyBinding::Unknown`
 * and the slot silently stops working, which is exactly how issue #20 read to
 * the user.
 */
describe("KeyInput", () => {
  const arm = async (value = "z") => {
    const onChange = vi.fn();
    render(<KeyInput label="Slot 1" value={value} onChange={onChange} />);
    await userEvent.click(screen.getByRole("button"));
    return { onChange, field: screen.getByRole("button") };
  };

  it("shows the current binding", () => {
    render(<KeyInput label="Slot 1" value="z" onChange={() => {}} />);
    expect(screen.getByRole("button")).toHaveTextContent("z");
  });

  it("shows Space as a word, not an empty box", () => {
    render(<KeyInput label="Slot 1" value="Space" onChange={() => {}} />);
    expect(screen.getByRole("button")).toHaveTextContent("Space");
  });

  it("sends the space bar as \"Space\"", async () => {
    const { onChange, field } = await arm();
    fireEvent.keyDown(field, { key: " " });
    expect(onChange).toHaveBeenCalledWith("Space");
  });

  it("lowercases a letter so enigo does not synthesize Shift+key", async () => {
    const { onChange, field } = await arm();
    fireEvent.keyDown(field, { key: "Z" });
    expect(onChange).toHaveBeenCalledWith("z");
  });

  it("captures the thumb mouse buttons", async () => {
    const { onChange, field } = await arm();
    fireEvent.mouseDown(field, { button: 3 });
    expect(onChange).toHaveBeenCalledWith("Mouse4");
  });

  it("captures the middle mouse button", async () => {
    const { onChange, field } = await arm();
    fireEvent.mouseDown(field, { button: 1 });
    expect(onChange).toHaveBeenCalledWith("Mouse3");
  });

  it("refuses left and right click — Dota needs them", async () => {
    const { onChange, field } = await arm();
    fireEvent.mouseDown(field, { button: 0 });
    fireEvent.mouseDown(field, { button: 2 });
    expect(onChange).not.toHaveBeenCalled();
  });

  it("ignores a modifier pressed on its own and stays armed", async () => {
    const { onChange, field } = await arm();

    fireEvent.keyDown(field, { key: "Shift" });
    expect(onChange).not.toHaveBeenCalled();

    // Still listening, so the key the user was reaching for still lands.
    fireEvent.keyDown(field, { key: "F1" });
    expect(onChange).toHaveBeenCalledWith("F1");
  });

  it("names numpad digits distinctly from the number row", async () => {
    const { onChange, field } = await arm();
    fireEvent.keyDown(field, { key: "7", location: 3 });
    expect(onChange).toHaveBeenCalledWith("Numpad7");
  });

  it("uses the canonical arrow names", async () => {
    const { onChange, field } = await arm();
    fireEvent.keyDown(field, { key: "ArrowUp" });
    expect(onChange).toHaveBeenCalledWith("Up");
  });

  it("ignores input until the field is armed", () => {
    const onChange = vi.fn();
    render(<KeyInput label="Slot 1" value="z" onChange={onChange} />);

    fireEvent.keyDown(screen.getByRole("button"), { key: "q" });
    fireEvent.mouseDown(screen.getByRole("button"), { button: 3 });

    expect(onChange).not.toHaveBeenCalled();
  });
});
