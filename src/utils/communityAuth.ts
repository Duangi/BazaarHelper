import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';

const COMMUNITY_TOOLS_URL = 'https://www.duang.work/tools';

interface GeneratedLoginKey {
  key: string;
  username: string;
  account_id: string;
}

export interface CommunityAutoLoginResult {
  url: string;
  username: string;
  accountId: string;
}

export async function buildCommunityAutoLoginUrl(baseUrl = COMMUNITY_TOOLS_URL): Promise<CommunityAutoLoginResult> {
  const result = await invoke<GeneratedLoginKey>('generate_game_login_key');
  const url = new URL(baseUrl);
  url.searchParams.set('authkey', result.key);
  return {
    url: url.toString(),
    username: result.username,
    accountId: result.account_id,
  };
}

export async function openCommunityWithAutoLogin(baseUrl = COMMUNITY_TOOLS_URL): Promise<CommunityAutoLoginResult> {
  const payload = await buildCommunityAutoLoginUrl(baseUrl);
  await openUrl(payload.url);
  return payload;
}
