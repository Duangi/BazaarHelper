import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { openCommunityWithAutoLogin } from '../../utils/communityAuth';

interface ProfilePanelProps {
  visible: boolean;
  inline?: boolean;
  onClose: () => void;
  showToast: (message: string, type?: 'success' | 'error' | 'warning' | 'info') => void;
}

interface GameIdentityInfo {
  username: string;
  account_id: string;
  steam_id?: string | null;
}

interface GeneratedLoginKey {
  key: string;
  username: string;
  account_id: string;
}

const maskAccountId = (value: string) => {
  if (!value) return '***';
  if (value.length <= 8) return `${value.slice(0, 2)}***${value.slice(-2)}`;
  return `${value.slice(0, 4)}***${value.slice(-4)}`;
};

export function ProfilePanel({ visible, inline = false, onClose, showToast }: ProfilePanelProps) {
  const [loading, setLoading] = useState(false);
  const [identity, setIdentity] = useState<GameIdentityInfo | null>(null);
  const [showAccountId, setShowAccountId] = useState(false);
  const [keyLoading, setKeyLoading] = useState(false);
  const [openCommunityLoading, setOpenCommunityLoading] = useState(false);
  const [loginKey, setLoginKey] = useState('');

  const loadIdentity = useCallback(async () => {
    setLoading(true);
    try {
      const data = await invoke<GameIdentityInfo>('get_game_identity');
      setIdentity(data);
    } catch (error) {
      const message = typeof error === 'string' ? error : String(error);
      showToast(`读取账号信息失败：${message}`, 'error');
    } finally {
      setLoading(false);
    }
  }, [showToast]);

  useEffect(() => {
    if (!visible) return;
    void loadIdentity();
  }, [loadIdentity, visible]);

  const accountText = useMemo(() => {
    if (!identity?.account_id) return '***';
    return showAccountId ? identity.account_id : maskAccountId(identity.account_id);
  }, [identity?.account_id, showAccountId]);

  const handleGenerateKey = useCallback(async () => {
    setKeyLoading(true);
    try {
      const result = await invoke<GeneratedLoginKey>('generate_game_login_key');
      setLoginKey(result.key);
      showToast('密钥已生成', 'success');
    } catch (error) {
      const message = typeof error === 'string' ? error : String(error);
      showToast(`生成密钥失败：${message}`, 'error');
    } finally {
      setKeyLoading(false);
    }
  }, [showToast]);

  const handleCopy = useCallback(async (value: string, label: string) => {
    if (!value) {
      showToast(`${label}为空`, 'warning');
      return;
    }
    try {
      await navigator.clipboard.writeText(value);
      showToast(`${label}已复制`, 'success');
    } catch {
      showToast(`复制${label}失败`, 'error');
    }
  }, [showToast]);

  const handleOpenCommunity = useCallback(async () => {
    setOpenCommunityLoading(true);
    try {
      await openCommunityWithAutoLogin();
      showToast('已打开网页并自动登录', 'success');
    } catch (error) {
      const message = typeof error === 'string' ? error : String(error);
      showToast(`打开网页失败：${message}`, 'error');
    } finally {
      setOpenCommunityLoading(false);
    }
  }, [showToast]);

  if (!visible) return null;

  return (
    <div className={`settings-panel ${inline ? 'inline' : ''}`} onClick={(e) => e.stopPropagation()}>
      <div className="settings-header">
        <h3>个人页面</h3>
        {!inline ? <button className="close-panel-btn" onClick={onClose}>×</button> : null}
      </div>

      <div className="settings-content" data-no-drag>
        <div className="setting-item">
          <div className="profile-row-header">
            <label>游戏账户名 Username</label>
            <button className="bulk-btn" style={{ padding: '4px 10px' }} onClick={() => void loadIdentity()}>
              {loading ? '读取中...' : '刷新'}
            </button>
          </div>
          <div className="profile-identity-card">
            <div className="profile-identity-title">当前账户</div>
            <div className="profile-identity-name">{identity?.username || '--'}</div>
            <div className="profile-identity-meta">
              AccountId: {accountText}
            </div>
          </div>
        </div>

        <div className="setting-item">
          <div className="profile-row-header">
            <label>AccountId</label>
            <div className="profile-btn-row">
              <button
                className="bulk-btn"
                style={{ padding: '4px 10px' }}
                onClick={() => setShowAccountId((prev) => !prev)}
                title={showAccountId ? '隐藏' : '显示'}
              >
                {showAccountId ? '🙈' : '👁️'}
              </button>
              <button
                className="bulk-btn"
                style={{ padding: '4px 10px' }}
                onClick={() => void handleCopy(identity?.account_id || '', 'AccountId')}
              >
                复制
              </button>
            </div>
          </div>
          <div className="profile-account-value">
            {accountText}
          </div>
          <div className="setting-tip">默认掩码显示，点击眼睛图标可查看真实值。</div>
        </div>

        <div className="setting-item">
          <div className="profile-row-header">
            <label>登录密钥</label>
            <div className="profile-btn-row">
              <button className="bulk-btn" style={{ padding: '4px 10px' }} onClick={() => void handleGenerateKey()}>
                {keyLoading ? '生成中...' : '生成密钥'}
              </button>
              <button className="bulk-btn" style={{ padding: '4px 10px' }} onClick={() => void handleCopy(loginKey, '密钥')}>
                复制
              </button>
            </div>
          </div>
          <div className="profile-login-key-box">
            {loginKey || '点击“生成密钥”后在这里显示'}
          </div>
          <div className="setting-tip">该密钥为插件侧可逆编码，不是强安全认证凭证；建议服务端再叠加时效和签名校验。</div>
        </div>

        <div className="setting-item">
          <div className="profile-row-header">
            <label>社区联动</label>
            <button className="bulk-btn" style={{ padding: '4px 12px' }} onClick={() => void handleOpenCommunity()}>
              {openCommunityLoading ? '打开中...' : '打开社区并登录'}
            </button>
          </div>
          <div className="setting-tip">会使用当前游戏账号生成一次性登录链接并打开 duang.work。</div>
        </div>
      </div>
    </div>
  );
}
