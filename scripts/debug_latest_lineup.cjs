const fs = require('fs');
const path = require('path');

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

const root = path.resolve(__dirname, '..');
const itemsPath = path.join(root, 'src-tauri', 'resources', 'items_db.json');
const items = readJson(itemsPath);
const nameMap = new Map();
for (const item of items) {
  if (!item || !item.id) continue;
  nameMap.set(item.id, {
    cn: item.name_cn || item.name_en || item.id,
    en: item.name_en || item.name_cn || item.id,
  });
}

const home = process.env.USERPROFILE || '';
const logPaths = [
  path.join(home, 'AppData', 'LocalLow', 'Tempo Storm', 'The Bazaar', 'Player-prev.log'),
  path.join(home, 'AppData', 'LocalLow', 'Tempo Storm', 'The Bazaar', 'Player.log'),
].filter((p) => fs.existsSync(p));

if (logPaths.length === 0) {
  console.log('NO_LOG_FILES');
  process.exit(0);
}

const reState = /State changed from \[.*?\] to \[(?<state>[^\]]+)\]/;
const rePurchase = /Card Purchased: InstanceId:\s*(?<iid>[^ ]+)\s*-\s*TemplateId\s*(?<tid>[^ ]+)/;
const reTarget = /Target:(?<tgt>[^ ]+)/;
const reMoved = /Successfully moved card\s+(?<iid>itm_[^ ]+)\s+to\s+(?<tgt>[^ ]+)/;
const reSold = /Sold Card\s+(?<iid>itm_[^ ]+)/;
const reRemoved = /Successfully removed item\s+(?<iid>itm_[^ ]+)/;
const reId = /ID: \[(?<id>[^\]]+)\]/;
const reTid = /TemplateId: \[(?<tid>[^\]]+)\]/;
const reOwner = /- Owner: \[(?<owner>[^\]]+)\]/;
const reSection = /- Section: \[(?<sec>[^\]]+)\]/;
const reDisposed = /itm_[A-Za-z0-9_-]+/g;

let instToTemp = new Map();
let hand = new Set();
let stash = new Set();
let inPvp = false;
let latestDay = 1;
let latestReplay = null;
let latestPvpStart = null;
let lastIid = '';
let curOwner = '';

for (const logPath of logPaths) {
  const content = fs.readFileSync(logPath, 'utf8');
  const lines = content.split(/\r?\n/);

  for (const raw of lines) {
    const line = raw.trim();

    if (line.includes('NetMessageRunInitialized') || line.includes('[GameInstance] Starting new run...')) {
      instToTemp = new Map();
      hand = new Set();
      stash = new Set();
      inPvp = false;
      latestDay = 1;
      latestReplay = null;
      lastIid = '';
      curOwner = '';
      continue;
    }

    const stateMatch = line.match(reState);
    if (stateMatch?.groups?.state) {
      const nextState = stateMatch.groups.state;
      if (nextState === 'PVPCombatState') {
        inPvp = true;
        const ordered = Array.from(hand).sort();
        const cards = ordered
          .map((iid) => {
            const tid = instToTemp.get(iid);
            if (!tid) return null;
            const n = nameMap.get(tid) || { cn: tid, en: tid };
            return { iid, tid, cn: n.cn, en: n.en };
          })
          .filter(Boolean);
        latestPvpStart = { day: latestDay, cards };
      }
      if (inPvp && (nextState === 'ChoiceState' || nextState === 'LevelUpState')) {
        latestDay += 1;
        inPvp = false;
      }
      if (nextState === 'ReplayState') {
        const ordered = Array.from(hand).sort();
        const cards = ordered
          .map((iid) => {
            const tid = instToTemp.get(iid);
            if (!tid) return null;
            const n = nameMap.get(tid) || { cn: tid, en: tid };
            return { iid, tid, cn: n.cn, en: n.en };
          })
          .filter(Boolean);
        latestReplay = { day: latestDay, cards };
      }
    }

    const p = line.match(rePurchase);
    if (p?.groups?.iid && p?.groups?.tid) {
      const iid = p.groups.iid;
      const tid = p.groups.tid;
      instToTemp.set(iid, tid);
      const target = line.match(reTarget)?.groups?.tgt || '';
      if (target.includes('PlayerStorageSocket')) {
        stash.add(iid);
        hand.delete(iid);
      } else if (target.includes('PlayerSocket')) {
        hand.add(iid);
        stash.delete(iid);
      }
    }

    const m = line.match(reMoved);
    if (m?.groups?.iid && m?.groups?.tgt) {
      const iid = m.groups.iid;
      const tgt = m.groups.tgt;
      if (tgt.includes('StorageSocket')) {
        stash.add(iid);
        hand.delete(iid);
      } else if (tgt.includes('Socket')) {
        hand.add(iid);
        stash.delete(iid);
      }
    }

    const sold = line.match(reSold)?.groups?.iid;
    if (sold) {
      hand.delete(sold);
      stash.delete(sold);
    }

    const removed = line.match(reRemoved)?.groups?.iid;
    if (removed) {
      hand.delete(removed);
      stash.delete(removed);
    }

    if (line.includes('Cards Disposed:')) {
      const matches = line.match(reDisposed) || [];
      for (const iid of matches) {
        hand.delete(iid);
        stash.delete(iid);
      }
    }

    const idMatch = line.match(reId)?.groups?.id;
    if (idMatch) {
      lastIid = idMatch;
    } else {
      const tidMatch = line.match(reTid)?.groups?.tid;
      if (tidMatch && lastIid) {
        instToTemp.set(lastIid, tidMatch);
      }
      const owner = line.match(reOwner)?.groups?.owner;
      if (owner) {
        curOwner = owner;
      }
      const sec = line.match(reSection)?.groups?.sec;
      if (sec) {
        if (lastIid && curOwner === 'Player' && lastIid.startsWith('itm_')) {
          if (sec === 'Hand' || sec === 'Player') {
            hand.add(lastIid);
            stash.delete(lastIid);
          } else if (sec === 'Stash' || sec === 'Storage' || sec === 'PlayerStorage') {
            stash.add(lastIid);
            hand.delete(lastIid);
          } else {
            hand.delete(lastIid);
            stash.delete(lastIid);
          }
        }
        lastIid = '';
        curOwner = '';
      }
    }
  }
}

if (!latestReplay) {
  if (!latestPvpStart) {
    console.log('NO_REPLAY_CAPTURED');
    process.exit(0);
  }
  console.log(`LATEST_PVP_START_DAY=${latestPvpStart.day}`);
  for (const c of latestPvpStart.cards.slice(0, 16)) {
    console.log(`${c.iid} | ${c.tid} | ${c.cn} | ${c.en}`);
  }
  const expected = ['永恒火炬', '魔法石', '炼金梨缶', '打火机'];
  const hits = expected.filter((name) => latestPvpStart.cards.some((c) => `${c.cn}${c.en}`.includes(name)));
  console.log(`EXPECTED_HITS=${hits.length}/4 -> ${hits.join(', ')}`);
  process.exit(0);
}

console.log(`LATEST_DAY=${latestReplay.day}`);
for (const c of latestReplay.cards.slice(0, 16)) {
  console.log(`${c.iid} | ${c.tid} | ${c.cn} | ${c.en}`);
}

const expected = ['永恒火炬', '魔法石', '炼金梨缶', '打火机'];
const hits = expected.filter((name) => latestReplay.cards.some((c) => `${c.cn}${c.en}`.includes(name)));
console.log(`EXPECTED_HITS=${hits.length}/4 -> ${hits.join(', ')}`);
