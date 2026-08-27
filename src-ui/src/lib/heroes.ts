/**
 * Hero slugs to the names and pictures a player recognises.
 *
 * The draft reader and STRATZ both key on Dota's internal slugs, which are
 * not what anyone calls these heroes: `necrolyte` is Necrophos, `zuus` is
 * Zeus, `wisp` is Io. Title-casing the slug — which the Lineup panel used to
 * do — puts "Necrolyte" on screen three inches from advice that says
 * "Necrophos" and makes the two panels look like they disagree.
 *
 * The exceptions below are every hero whose slug does not title-case into its
 * own name, taken from the STRATZ hero constants rather than typed by hand
 * (`cargo run --release --example stratz_meta_probe` prints the list).
 */
const DISPLAY_NAMES: Record<string, string> = {
  abyssal_underlord: "Underlord",
  antimage: "Anti-Mage",
  centaur: "Centaur Warrunner",
  doom_bringer: "Doom",
  furion: "Nature's Prophet",
  keeper_of_the_light: "Keeper of the Light",
  life_stealer: "Lifestealer",
  magnataur: "Magnus",
  necrolyte: "Necrophos",
  nevermore: "Shadow Fiend",
  obsidian_destroyer: "Outworld Destroyer",
  queenofpain: "Queen of Pain",
  rattletrap: "Clockwerk",
  shredder: "Timbersaw",
  skeleton_king: "Wraith King",
  treant: "Treant Protector",
  vengefulspirit: "Vengeful Spirit",
  windrunner: "Windranger",
  wisp: "Io",
  zuus: "Zeus",
};

/** Strip the prefix GSI uses; the reader's own slugs arrive without it. */
function bareSlug(slug: string): string {
  return slug.replace("npc_dota_hero_", "").trim();
}

/** `necrolyte` -> `Necrophos`, `skywrath_mage` -> `Skywrath Mage`. */
export function heroName(slug: string): string {
  const bare = bareSlug(slug);
  return (
    DISPLAY_NAMES[bare] ??
    bare
      .split("_")
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(" ")
  );
}

/**
 * Valve's own portrait for a hero.
 *
 * Remote rather than bundled: the pack is ~12MB of PNGs for a panel that only
 * appears while STRATZ is already being queried, so the network is up anyway.
 * Every caller must degrade to text if the image does not load — see
 * `HeroPortrait`.
 */
export function heroPortraitUrl(slug: string): string {
  return `https://cdn.cloudflare.steamstatic.com/apps/dota2/images/dota_react/heroes/${bareSlug(
    slug,
  )}.png`;
}
