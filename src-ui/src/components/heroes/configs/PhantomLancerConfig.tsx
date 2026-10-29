import { Card } from "../../common/Card";
import { Toggle } from "../../common/Toggle";
import { NumberInput } from "../../common/NumberInput";
import { Slider } from "../../common/Slider";
import { KeyInput } from "../../common/KeyInput";
import { useConfigStore } from "../../../stores/configStore";

export default function PhantomLancerConfig() {
  const config = useConfigStore((s) => s.config.heroes.phantom_lancer);
  const update = useConfigStore((s) => s.updateHeroConfig);
  const set = (updates: Partial<typeof config>) => update("phantom_lancer", updates);

  return (
    <>
      <div className="space-y-4">
        <Card title="Ctrl Latch">
          <Toggle label="Enable Ctrl Latch" checked={config.ctrl_latch_enabled} onChange={(v) => set({ ctrl_latch_enabled: v })} />
          <NumberInput label="Tap Threshold" value={config.latch_tap_ms} onChange={(v) => set({ latch_tap_ms: v })} suffix="ms" />
          <NumberInput label="Ctrl Gap" value={config.latch_ctrl_gap_ms} onChange={(v) => set({ latch_ctrl_gap_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            Tap Ctrl and every right-click becomes a unified order, so all your
            illusions follow it. Tap again to stop. Holding Ctrl, or Ctrl with
            any other key or click, is your own Ctrl and never toggles it. It
            turns off when you die or use the illusion send.
          </p>
          <p className="text-xs text-muted">
            Only the right-click gets Ctrl. Abilities, items and number keys
            never do, so nothing levels up and no control group changes.
          </p>
          <p className="text-xs text-muted">
            The tap threshold is the longest Ctrl press that counts as a tap;
            natural taps measured 81–121ms. Raise the Ctrl gap if some illusions
            miss a latched order.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Illusion Send">
          <Toggle label="Enable Illusion Send" checked={config.illusion_send_enabled} onChange={(v) => set({ illusion_send_enabled: v })} />
          <KeyInput label="Trigger" value={config.illusion_send_trigger} onChange={(v) => set({ illusion_send_trigger: v })} />
          <KeyInput label="Select All Other Units Key" value={config.select_other_units_key} onChange={(v) => set({ select_other_units_key: v })} />
          <KeyInput label="Select Hero Key" value={config.select_hero_key} onChange={(v) => set({ select_hero_key: v })} />
          <NumberInput label="Step Delay" value={config.illusion_send_step_ms} onChange={(v) => set({ illusion_send_step_ms: v })} suffix="ms" />
          <p className="text-xs text-muted">
            One press selects every other unit, right-clicks at the cursor, and
            selects your hero again: the illusions go, you stay. It also turns
            the Ctrl latch off, so your next right-click moves only the hero.
          </p>
          <p className="text-xs text-muted">
            The two select keys must match your Dota hotkeys. The trigger is
            swallowed, so Dota never sees it — with the default middle mouse,
            that means no middle-mouse camera drag while playing Phantom Lancer.
            Raise the step delay if the illusions do not move.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Latch Sound">
          <Toggle label="Play On/Off Sound" checked={config.latch_cue_enabled} onChange={(v) => set({ latch_cue_enabled: v })} />
          <Slider label="Volume" value={config.latch_cue_volume} min={0} max={1} step={0.05} onChange={(v) => set({ latch_cue_volume: v })} />
          <p className="text-xs text-muted">
            Rising when the latch turns on, falling when it turns off. Dota
            shows nothing for it, so this is the only way to know which mode
            you are in.
          </p>
        </Card>
      </div>

      <div className="space-y-4">
        <Card title="Before You Use It">
          <p className="text-xs text-muted">
            <strong>Turn on "Unified Orders with Ctrl"</strong> in Dota's
            options. The latch only supplies the Ctrl; that option is what makes
            Ctrl send the order to every unit.
          </p>
          <p className="text-xs text-muted">
            Only active while Phantom Lancer is the active hero.
          </p>
        </Card>
      </div>
    </>
  );
}
