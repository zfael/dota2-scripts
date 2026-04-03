import { useEffect, useRef } from "react";

const ALERT_WINDOW_SECONDS = 10;

export function useRuneAlert(
  runeTimer: number | null,
  alertsEnabled: boolean,
  audioEnabled: boolean,
) {
  const lastAlertRef = useRef<number | null>(null);

  useEffect(() => {
    if (
      !alertsEnabled ||
      !audioEnabled ||
      runeTimer === null ||
      runeTimer > ALERT_WINDOW_SECONDS
    ) {
      lastAlertRef.current = null;
      return;
    }

    if (
      lastAlertRef.current !== null &&
      lastAlertRef.current <= ALERT_WINDOW_SECONDS
    ) {
      return;
    }

    lastAlertRef.current = runeTimer;

    try {
      const ctx = new AudioContext();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();

      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.frequency.value = 880;
      gain.gain.value = 0.15;
      osc.start();
      osc.stop(ctx.currentTime + 0.12);

      setTimeout(() => {
        void ctx.close();
      }, 500);
    } catch {
      // AudioContext may not be available.
    }
  }, [alertsEnabled, audioEnabled, runeTimer]);
}
