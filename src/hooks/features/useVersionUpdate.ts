import { useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { type Update } from '@tauri-apps/plugin-updater';

type UpdateStatus = 'none' | 'checking' | 'available' | 'downloading' | 'ready';

export const useVersionUpdate = () => {
  const [showVersionScreen, setShowVersionScreen] = useState(true);
  const [currentVersion, setCurrentVersion] = useState('');
  const [updateAvailable, setUpdateAvailable] = useState<Update | null>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>('none');
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [isInstalling, setIsInstalling] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [announcement, setAnnouncement] = useState('');

  const enterApp = async () => {
    console.log('[Update] Entering App. updateAvailable:', !!updateAvailable);
    setShowVersionScreen(false);
    invoke('start_template_loading').catch(console.error);
    invoke('load_event_templates').catch(console.error);

    if (updateAvailable) {
      console.log('[Update] Found update, but entering app without auto-download (Manual Trigger Mode).');
    }
  };

  const startUpdateDownload = async () => {
    if (!updateAvailable) return;
    try {
      console.log('[Update] Starting download...');
      setUpdateStatus('downloading');
      setDownloadProgress(0);

      let contentLength = 0;
      let downloaded = 0;

      await updateAvailable.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength || 0;
            console.log(`[Update] Download started, total bytes: ${contentLength}`);
            setDownloadProgress(0);
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              const progress = Math.min(100, Math.round((downloaded / contentLength) * 100));
              setDownloadProgress(progress);
            }
            break;
          case 'Finished':
            console.log('[Update] Download finished');
            setUpdateStatus('ready');
            setDownloadProgress(100);
            break;
        }
      });
    } catch (e) {
      console.error('[Update] Download failed:', e);
      setUpdateStatus('available');
      setErrorMessage(`更新下载失败: ${e}`);
    }
  };

  useEffect(() => {
    const initApp = async () => {
      console.log('[App] initApp 开始执行...');

      try {
        const appVersion = await getVersion();
        setCurrentVersion(appVersion);
        console.log(`[App] 启动初始化。当前版本: v${appVersion}`);
      } catch (e) {
        console.error('获取版本失败:', e);
      }

      const fallbackNotice =
        '🧠 脑子是用来构筑的，数据交给小抄记。\n\n💡 这只是个免费的记牌小工具，又不是考研资料，谁要是敢收你的费，请反手给他一个大逼兜！👊\n\n🍖 本小抄由 B站@这是李Duang啊 免费发放，付费获取的同学请立刻退款买排骨吃！';

      fetch(`https://gh.llkk.cc/https://raw.githubusercontent.com/Duangi/BazaarHelper/main/update.json?t=${Date.now()}`)
        .then((res) => (res.ok ? res.json() : null))
        .then((data) => {
          if (data && data.notes) {
            setAnnouncement(`${data.notes}\n\n------------------\n\n${fallbackNotice}`);
          } else {
            setAnnouncement(fallbackNotice);
          }
        })
        .catch((err) => {
          console.error('[App] 获取公告失败:', err);
          setAnnouncement(fallbackNotice);
        });

      // Disable startup auto-check to avoid unexpected exits on some macOS environments.
      // Users can still manually check updates from settings.
      setUpdateStatus('none');
    };

    void initApp();
  }, []);

  return {
    announcement,
    currentVersion,
    downloadProgress,
    enterApp,
    errorMessage,
    isInstalling,
    setAnnouncement,
    setErrorMessage,
    setIsInstalling,
    setUpdateAvailable,
    setUpdateStatus,
    showVersionScreen,
    startUpdateDownload,
    updateAvailable,
    updateStatus,
  };
};
