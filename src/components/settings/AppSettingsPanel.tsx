import { memo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { SettingGroup } from '../SettingsPanel';
import { SponsorSection } from './SponsorSection';
import { HotkeyCaptureOverlay } from './HotkeyCaptureOverlay';
import { ResetHotkeysConfirmModal } from './ResetHotkeysConfirmModal';
import { getHotkeyLabel } from '../../utils/helpers';

interface SettingsExpandedState {
  ui: boolean;
  yolo: boolean;
  hotkeys: boolean;
  debug: boolean;
}

import type { Dispatch, SetStateAction } from 'react';

interface AppSettingsPanelProps {
  visible: boolean;
  inline?: boolean;
  onClose: () => void;
  settingsExpanded: SettingsExpandedState;
  setSettingsExpanded: Dispatch<SetStateAction<SettingsExpandedState>>;
  fontSize: number;
  setFontSize: Dispatch<SetStateAction<number>>;
  setExpandedWidth: Dispatch<SetStateAction<number>>;
  setExpandedHeight: Dispatch<SetStateAction<number>>;
  setHasCustomPosition: Dispatch<SetStateAction<boolean>>;
  showToast: (message: string, type?: 'success' | 'error' | 'warning' | 'info') => void;
  enableYoloAuto: boolean;
  setEnableYoloAuto: Dispatch<SetStateAction<boolean>>;
  useGpuAcceleration: boolean;
  setUseGpuAcceleration: Dispatch<SetStateAction<boolean>>;
  yoloScanInterval: number;
  setYoloScanInterval: Dispatch<SetStateAction<number>>;
  showYoloMonitor: boolean;
  setShowYoloMonitor: Dispatch<SetStateAction<boolean>>;
  yoloHotkey: number | null;
  setYoloHotkey: Dispatch<SetStateAction<number | null>>;
  isRecordingYoloHotkey: boolean;
  setIsRecordingYoloHotkey: Dispatch<SetStateAction<boolean>>;
  detailDisplayHotkey: number | null;
  setDetailDisplayHotkey: Dispatch<SetStateAction<number | null>>;
  isRecordingDetailHotkey: boolean;
  setIsRecordingDetailHotkey: Dispatch<SetStateAction<boolean>>;
  detectionHotkey: number | null;
  setDetectionHotkey: Dispatch<SetStateAction<number | null>>;
  isRecordingHotkey: boolean;
  setIsRecordingHotkey: Dispatch<SetStateAction<boolean>>;
  cardDetectionHotkey: number | null;
  setCardDetectionHotkey: Dispatch<SetStateAction<number | null>>;
  isRecordingCardHotkey: boolean;
  setIsRecordingCardHotkey: Dispatch<SetStateAction<boolean>>;
  toggleCollapseHotkey: number | null;
  setToggleCollapseHotkey: Dispatch<SetStateAction<number | null>>;
  isRecordingToggleHotkey: boolean;
  setIsRecordingToggleHotkey: Dispatch<SetStateAction<boolean>>;
  showResetHotkeysConfirm: boolean;
  setShowResetHotkeysConfirm: Dispatch<SetStateAction<boolean>>;
  onConfirmResetHotkeys: () => Promise<void>;
  currentVersion: string;
  updateStatus: string;
  downloadProgress: number;
  updateAvailableVersion?: string;
  onManualCheckUpdate: () => Promise<void>;
  onStartUpdateDownload: () => void;
  onInstallReady: () => void;
  announcement: string;
  sponsorIcons: { vx: string; zfb: string };
}

function AppSettingsPanelImpl(props: AppSettingsPanelProps) {
  const {
    visible,
    inline = false,
    onClose,
    settingsExpanded,
    setSettingsExpanded,
    fontSize,
    setFontSize,
    setExpandedWidth,
    setExpandedHeight,
    setHasCustomPosition,
    showToast,
    enableYoloAuto,
    setEnableYoloAuto,
    useGpuAcceleration,
    setUseGpuAcceleration,
    yoloScanInterval,
    setYoloScanInterval,
    showYoloMonitor: _showYoloMonitor,
    setShowYoloMonitor: _setShowYoloMonitor,
    yoloHotkey,
    setYoloHotkey,
    isRecordingYoloHotkey,
    setIsRecordingYoloHotkey,
    detailDisplayHotkey,
    setDetailDisplayHotkey,
    isRecordingDetailHotkey,
    setIsRecordingDetailHotkey,
    detectionHotkey,
    setDetectionHotkey,
    isRecordingHotkey,
    setIsRecordingHotkey,
    cardDetectionHotkey,
    setCardDetectionHotkey,
    isRecordingCardHotkey,
    setIsRecordingCardHotkey,
    toggleCollapseHotkey,
    setToggleCollapseHotkey,
    isRecordingToggleHotkey,
    setIsRecordingToggleHotkey,
    showResetHotkeysConfirm,
    setShowResetHotkeysConfirm,
    onConfirmResetHotkeys,
    currentVersion,
    updateStatus,
    downloadProgress,
    updateAvailableVersion,
    onManualCheckUpdate,
    onStartUpdateDownload,
    onInstallReady,
    announcement,
    sponsorIcons,
  } = props;

  if (!visible) return null;

  const panel = (
    <div className={`settings-panel ${inline ? 'inline' : ''}`} onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <h3>设置</h3>
          {!inline ? <button className="close-panel-btn" onClick={onClose}>×</button> : null}
        </div>

        <div className="settings-content" data-no-drag>
          <SettingGroup
            title="⚙️ 界面设置"
            expanded={settingsExpanded.ui}
            onToggle={() => setSettingsExpanded((prev) => ({ ...prev, ui: !prev.ui }))}
          >
            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <label>字体大小: {fontSize}px</label>
                <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={() => {
                  setFontSize(16);
                  localStorage.setItem('user-font-size', '16');
                  showToast('字体大小已重置', 'success');
                }}>重置</button>
              </div>
              <input
                type="range"
                min="10"
                max="32"
                value={fontSize}
                onChange={(e) => {
                  const val = parseInt(e.target.value, 10);
                  setFontSize(val);
                  localStorage.setItem('user-font-size', val.toString());
                }}
              />
            </div>

            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                <label>窗口布局</label>
                <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={() => {
                  localStorage.removeItem('plugin-width');
                  localStorage.removeItem('plugin-height');
                  setExpandedWidth(400);
                  setExpandedHeight(700);
                  setHasCustomPosition(false);
                  showToast('窗口布局已重置', 'success');
                }}>重置宽高与位置</button>
              </div>
              <div className="setting-tip">调整后将实时影响所有文字大小</div>
            </div>

            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label>详情弹窗位置</label>
                <button className="bulk-btn" style={{ padding: '4px 12px' }} onClick={async () => {
                  try {
                    await invoke('reset_detail_popup_position');
                    showToast('详情弹窗位置已重置', 'success');
                  } catch (e) {
                    console.error('Failed to reset detail popup position:', e);
                    showToast('重置失败', 'error');
                  }
                }}>重置位置</button>
              </div>
              <div style={{ fontSize: '11px', color: '#888', marginTop: '8px' }}>
                重置详情弹窗到默认位置（鼠标所在屏幕的中心）
              </div>
            </div>
          </SettingGroup>

          <SettingGroup
            title="🔍 YOLO设置"
            expanded={settingsExpanded.yolo}
            onToggle={() => setSettingsExpanded((prev) => ({ ...prev, yolo: !prev.yolo }))}
          >
            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label>YOLO自动识别</label>
                <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                  {enableYoloAuto && (
                    <button
                      className="bulk-btn"
                      style={{
                        padding: '4px 12px',
                        background: useGpuAcceleration ? 'rgba(76, 175, 80, 0.2)' : 'rgba(244, 67, 54, 0.2)',
                        borderColor: useGpuAcceleration ? '#4CAF50' : '#f44336',
                        color: useGpuAcceleration ? '#4CAF50' : '#f44336',
                      }}
                      onClick={() => {
                        const newVal = !useGpuAcceleration;
                        setUseGpuAcceleration(newVal);
                        localStorage.setItem('use-gpu-acceleration', newVal.toString());
                        showToast(`GPU加速已${newVal ? '开启' : '关闭'}`, 'info');
                      }}
                    >
                      GPU加速: {useGpuAcceleration ? '开' : '关'}
                    </button>
                  )}
                  <button
                    className="bulk-btn"
                    style={{
                      padding: '4px 12px',
                      background: enableYoloAuto ? 'rgba(76, 175, 80, 0.2)' : 'rgba(244, 67, 54, 0.2)',
                      borderColor: enableYoloAuto ? '#4CAF50' : '#f44336',
                      color: enableYoloAuto ? '#4CAF50' : '#f44336',
                    }}
                    onClick={() => {
                      const newVal = !enableYoloAuto;
                      setEnableYoloAuto(newVal);
                      localStorage.setItem('enable-yolo-auto', newVal.toString());
                      showToast(`YOLO自动识别已${newVal ? '开启' : '关闭'}`, 'info');
                    }}
                  >
                    {enableYoloAuto ? '已开启' : '已关闭'}
                  </button>
                </div>
              </div>
              <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                启用后每隔固定时间自动触发YOLO识别卡牌（下方可调整频率）
              </div>
            </div>

            <div className="setting-item" style={{ opacity: enableYoloAuto ? 1 : 0.5 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label>YOLO扫描频率</label>
                <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                  <input
                    type="range"
                    min="0.5"
                    max="2"
                    step="0.1"
                    value={yoloScanInterval}
                    disabled={!enableYoloAuto}
                    onChange={(e) => {
                      const newVal = parseFloat(e.target.value);
                      setYoloScanInterval(newVal);
                      localStorage.setItem('yolo-scan-interval', newVal.toString());
                    }}
                    style={{ width: '120px', accentColor: '#ffcd19' }}
                  />
                  <span style={{ fontSize: '13px', color: '#ffcd19', fontWeight: 'bold', minWidth: '50px' }}>
                    {yoloScanInterval.toFixed(1)}s
                  </span>
                </div>
              </div>
              <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                设置YOLO自动识别的时间间隔（0.5秒 - 2秒）
              </div>
            </div>

            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label>YOLO结果展示</label>
                <span style={{ fontSize: '12px', color: '#ffcd19', fontWeight: 600 }}>灵动岛概览</span>
              </div>
              <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                YOLO监控独立窗口已停用。手动触发或自动扫描结果会显示在收起态灵动岛。
              </div>
            </div>

            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label>图片缓存清理</label>
                <button
                  className="bulk-btn"
                  style={{ padding: '4px 12px' }}
                  onClick={async () => {
                    try {
                      const report = await invoke<{
                        removed_dirs: number;
                        removed_files: number;
                        removed_bytes: number;
                      }>('clear_generated_image_caches');
                      const mb = report.removed_bytes / (1024 * 1024);
                      showToast(
                        `已清理 ${report.removed_files} 个文件（${mb.toFixed(2)} MB）`,
                        'success',
                      );
                    } catch (e) {
                      console.error('Failed to clear generated image caches:', e);
                      showToast('清理失败，请查看日志', 'error');
                    }
                  }}
                >
                  清理缓存图片
                </button>
              </div>
              <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                清理搜索缩略图与调试截图缓存文件，避免长期堆积占用存储。
              </div>
            </div>

            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label>YOLO手动触发快捷键</label>
                <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={(e) => { e.preventDefault(); setIsRecordingYoloHotkey(true); }}>
                  {isRecordingYoloHotkey ? '请按键...' : (yoloHotkey ? getHotkeyLabel(yoloHotkey) : '未设置')}
                </button>
              </div>
              <HotkeyCaptureOverlay
                visible={isRecordingYoloHotkey}
                description="支持: 键盘按键, 鼠标中键/侧键（不支持左右键）"
                allowMouseButtons={[1, 3, 4]}
                onCapture={(vk) => {
                  setYoloHotkey(vk);
                  localStorage.setItem('yolo-hotkey', vk.toString());
                  setIsRecordingYoloHotkey(false);
                }}
                onCancel={() => setIsRecordingYoloHotkey(false)}
              />
              <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                按此键立即触发YOLO识别（默认: 未设置）
              </div>
            </div>

            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label>卡牌详情显示按键</label>
                <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={(e) => { e.preventDefault(); setIsRecordingDetailHotkey(true); }}>
                  {isRecordingDetailHotkey ? '请按键...' : (detailDisplayHotkey ? getHotkeyLabel(detailDisplayHotkey) : '未设置')}
                </button>
              </div>
              <HotkeyCaptureOverlay
                visible={isRecordingDetailHotkey}
                description="支持: 键盘按键, 鼠标左/中/右键/侧键"
                onCapture={(vk) => {
                  setDetailDisplayHotkey(vk);
                  invoke('set_detail_display_hotkey', { hotkey: vk });
                  setIsRecordingDetailHotkey(false);
                }}
                onCancel={() => setIsRecordingDetailHotkey(false)}
              />
              <div style={{ fontSize: '11px', color: '#888', marginTop: '4px' }}>
                按此键显示鼠标位置的卡牌/怪物/事件详情（默认: 未设置）
              </div>
            </div>
          </SettingGroup>

          <SettingGroup
            title="⌨️ 快捷键设置"
            expanded={settingsExpanded.hotkeys}
            onToggle={() => setSettingsExpanded((prev) => ({ ...prev, hotkeys: !prev.hotkeys }))}
          >
            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                <label>怪物识别按键</label>
                <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={(e) => { e.preventDefault(); setIsRecordingHotkey(true); }}>
                  {isRecordingHotkey ? '请按键...' : (detectionHotkey ? getHotkeyLabel(detectionHotkey) : '未设置')}
                </button>
              </div>
              <HotkeyCaptureOverlay
                visible={isRecordingHotkey}
                description="支持: 键盘按键, 鼠标左/中/右键/侧键"
                onCapture={(vk) => {
                  setDetectionHotkey(vk);
                  invoke('set_detection_hotkey', { hotkey: vk });
                  setIsRecordingHotkey(false);
                }}
                onCancel={() => setIsRecordingHotkey(false)}
              />
              <div className="setting-tip">默认: 未设置</div>
            </div>

            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                <label>卡牌识别按键</label>
                <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={(e) => { e.preventDefault(); setIsRecordingCardHotkey(true); }}>
                  {isRecordingCardHotkey ? '请按键...' : (cardDetectionHotkey ? getHotkeyLabel(cardDetectionHotkey) : '未设置')}
                </button>
              </div>
              <HotkeyCaptureOverlay
                visible={isRecordingCardHotkey}
                description="支持: 键盘按键, 鼠标左/中/右键/侧键"
                onCapture={(vk) => {
                  setCardDetectionHotkey(vk);
                  invoke('set_card_detection_hotkey', { hotkey: vk });
                  setIsRecordingCardHotkey(false);
                }}
                onCancel={() => setIsRecordingCardHotkey(false)}
              />
              <div className="setting-tip">默认: 未设置</div>
            </div>

            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                <label>一键收起/展开插件</label>
                <button className="bulk-btn" style={{ padding: '2px 8px' }} onClick={(e) => { e.preventDefault(); setIsRecordingToggleHotkey(true); }}>
                  {isRecordingToggleHotkey ? '请按键...' : (toggleCollapseHotkey ? getHotkeyLabel(toggleCollapseHotkey) : '未设置')}
                </button>
              </div>
              <HotkeyCaptureOverlay
                visible={isRecordingToggleHotkey}
                description="支持: 键盘按键, 鼠标中键/侧键（不支持左右键）"
                allowMouseButtons={[1, 3, 4]}
                onCapture={(vk) => {
                  setToggleCollapseHotkey(vk);
                  invoke('set_toggle_collapse_hotkey', { hotkey: vk });
                  setIsRecordingToggleHotkey(false);
                }}
                onCancel={() => setIsRecordingToggleHotkey(false)}
              />
              <div className="setting-tip">默认: 未设置</div>
            </div>

            <div className="setting-divider" style={{ borderTop: '1px solid rgba(255,255,255,0.1)', margin: '15px 0' }}></div>

            <div className="setting-item">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <label>快捷键管理</label>
                <button className="bulk-btn" style={{ padding: '4px 12px', background: 'rgba(255, 69, 58, 0.15)', borderColor: 'rgba(255, 69, 58, 0.4)' }} onClick={() => {
                  setShowResetHotkeysConfirm(true);
                }}>重置所有快捷键</button>
              </div>
              <div style={{ fontSize: '11px', color: '#888', marginTop: '8px' }}>
                将所有快捷键重置为"未设置"状态，禁用所有快捷键功能
              </div>
            </div>

            <ResetHotkeysConfirmModal
              visible={showResetHotkeysConfirm}
              onCancel={() => setShowResetHotkeysConfirm(false)}
              onConfirm={onConfirmResetHotkeys}
            />
          </SettingGroup>

          <div className="setting-divider" style={{ borderTop: '1px solid rgba(255,255,255,0.1)', margin: '15px 0' }}></div>

          <div className="setting-item">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
              <label>版本信息: v{currentVersion}</label>
              <button
                className="bulk-btn"
                style={{
                  padding: '2px 8px',
                  opacity: updateStatus === 'checking' ? 0.5 : 1,
                  cursor: updateStatus === 'checking' ? 'not-allowed' : 'pointer',
                }}
                disabled={updateStatus === 'checking' || updateStatus === 'downloading'}
                onClick={() => void onManualCheckUpdate()}
              >
                {updateStatus === 'checking' ? '检查中...' : '检查更新'}
              </button>
            </div>

            {updateStatus === 'checking' && <div style={{ fontSize: 'calc(12px * var(--font-scale, 1))', color: '#999' }}>正在检查远端更新...</div>}
            {updateStatus === 'none' && <div style={{ fontSize: 'calc(12px * var(--font-scale, 1))', color: '#238636' }}>当前已经是最新版本</div>}

            {(updateStatus === 'available' || updateStatus === 'downloading' || updateStatus === 'ready') && (
              <div style={{ background: 'rgba(56, 139, 253, 0.15)', border: '1px solid rgba(56, 139, 253, 0.4)', padding: '10px', borderRadius: '6px' }}>
                <div style={{ fontSize: 'calc(13px * var(--font-scale, 1))', fontWeight: 'bold', marginBottom: '8px', color: '#58a6ff' }}>
                  发现新版本: v{updateAvailableVersion}
                </div>

                {updateStatus === 'available' && (
                  <button className="bulk-btn" style={{ width: '100%', padding: '6px', background: '#238636', border: 'none' }} onClick={onStartUpdateDownload}>
                    立即下载更新
                  </button>
                )}

                {updateStatus === 'downloading' && (
                  <div>
                    <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '12px', marginBottom: '4px' }}>
                      <span>正在下载后台更新...</span>
                      <span>{downloadProgress}%</span>
                    </div>
                    <div style={{ background: 'rgba(255,255,255,0.1)', height: '4px', borderRadius: '2px' }}>
                      <div style={{ background: '#58a6ff', width: `${downloadProgress}%`, height: '100%', borderRadius: '2px', transition: 'width 0.3s' }}></div>
                    </div>
                  </div>
                )}

                {updateStatus === 'ready' && (
                  <button className="bulk-btn" style={{ width: '100%', padding: '6px', background: '#238636', border: 'none' }} onClick={onInstallReady}>
                    下载完成，点击重启安装
                  </button>
                )}
              </div>
            )}
          </div>

          {announcement && (
            <div className="setting-item" style={{ marginTop: '20px' }}>
              <label style={{ display: 'block', marginBottom: '8px', color: '#8b949e' }}>当前公告</label>
              <div className="settings-announcement-text">{announcement}</div>
            </div>
          )}

          <div className="setting-divider" style={{ borderTop: '1px solid rgba(255,255,255,0.1)', margin: '15px 0' }}></div>

          <div className="setting-item">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '8px' }}>
              <label>应用控制</label>
              <button
                className="bulk-btn"
                style={{
                  padding: '4px 12px',
                  background: 'rgba(244, 67, 54, 0.2)',
                  borderColor: '#f44336',
                  color: '#ff8a80',
                }}
                onClick={() => {
                  showToast('正在关闭应用...', 'info');
                  setTimeout(() => {
                    void invoke('request_app_exit');
                  }, 120);
                }}
              >
                关闭应用
              </button>
            </div>
          </div>

          <SponsorSection sponsorIcons={sponsorIcons} />
        </div>
      </div>
  );

  if (inline) {
    return <div className="settings-panel-inline-host">{panel}</div>;
  }

  return (
    <div className="settings-panel-overlay" onClick={onClose}>
      {panel}
    </div>
  );
}

export const AppSettingsPanel = memo(AppSettingsPanelImpl);
