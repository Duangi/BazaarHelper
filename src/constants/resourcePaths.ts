export const RESOURCE_DB_FILES = {
  items: 'items_db.json',
  skills: 'skills_db.json',
  monsters: 'monsters_db.json',
  eventEncounters: 'event_encounters.json',
  eventDetail: 'event_detail.json',
  combatEncounters: 'combat_encounters.json',
  cardIdMapping: 'card_id_mapping.json',
} as const;

export const RESOURCE_DIR_ALIASES: Array<{ canonical: string; legacy: string }> = [
  { canonical: 'assets/items', legacy: 'images' },
  { canonical: 'assets/skills', legacy: 'images/skill' },
  { canonical: 'assets/monsters/characters', legacy: 'images_monster_char' },
  { canonical: 'assets/monsters/backgrounds', legacy: 'images_monster_bg' },
  { canonical: 'assets/gui', legacy: 'images_GUI' },
  { canonical: 'assets/events/characters', legacy: 'EncEvent_CHAR' },
  { canonical: 'assets/events/backgrounds', legacy: 'EncEvent_BG' },
  { canonical: 'assets/events/icons', legacy: 'EncEvent_Icons' },
  { canonical: 'assets/sponsor', legacy: 'sponsor' },
];

export const normalizeResourcePath = (rawPath: string): string => {
  return rawPath.replace(/^resources\//, '').replace(/^\//, '');
};

const mapPrefix = (value: string, fromPrefix: string, toPrefix: string): string | null => {
  if (value === fromPrefix) return toPrefix;
  if (value.startsWith(`${fromPrefix}/`)) {
    return `${toPrefix}/${value.slice(fromPrefix.length + 1)}`;
  }
  return null;
};

export const buildResourceCandidates = (rawPath: string): string[] => {
  const normalized = normalizeResourcePath(rawPath);
  const candidates = new Set<string>([normalized]);

  for (const alias of RESOURCE_DIR_ALIASES) {
    const fromCanonical = mapPrefix(normalized, alias.canonical, alias.legacy);
    if (fromCanonical) candidates.add(fromCanonical);

    const fromLegacy = mapPrefix(normalized, alias.legacy, alias.canonical);
    if (fromLegacy) candidates.add(fromLegacy);
  }

  return [...candidates];
};
