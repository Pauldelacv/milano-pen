import { useEffect, useState } from 'react';
import { backupDatabase, generateBenchmark, restoreDatabase, run } from '../app/actions';
import { api } from '../services/ipc';
import { useUiStore } from '../stores/uiStore';
import type { AppInfo, Settings } from '../types/models';
import { Modal } from './Modal';

export function SettingsDialog() {
  const close = () => useUiStore.getState().setDialog(null);
  const settings = useUiStore((s) => s.settings);
  const [info, setInfo] = useState<AppInfo | null>(null);
  useEffect(() => {
    void run(api.appInfo()).then((i) => i && setInfo(i));
  }, []);
  const save = async (patch: Partial<Settings>) => {
    const next = { ...settings, ...patch };
    useUiStore.getState().setSettings(next);
    const saved = await run(api.saveSettings(next));
    if (saved) useUiStore.getState().setSettings(saved);
  };
  return (
    <Modal title="Settings" onClose={close}>
      <section className="settings">
        <label className="setting">
          <span>Appearance</span>
          <select value={settings.theme} onChange={(e) => void save({ theme: e.target.value as Settings['theme'] })}>
            <option value="system">Match system</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </label>
        <label className="setting">
          <span>
            Mouse wheel zooms
            <small>Off: the wheel scrolls the board; pinch or hold ⌘ to zoom.</small>
          </span>
          <input type="checkbox" checked={settings.wheelZooms} onChange={(e) => void save({ wheelZooms: e.target.checked })} />
        </label>
        <label className="setting">
          <span>
            Snap to grid
            <small>Hold ⌥ while dragging to place freely.</small>
          </span>
          <input type="checkbox" checked={settings.snapToGrid} onChange={(e) => void save({ snapToGrid: e.target.checked })} />
        </label>
        <label className="setting">
          <span>
            Daily automatic backup
            <small>Keeps the last 10 copies in the backups folder.</small>
          </span>
          <input type="checkbox" checked={settings.autoBackup} onChange={(e) => void save({ autoBackup: e.target.checked })} />
        </label>
      </section>
      <section className="settings">
        <h3>Your data</h3>
        <p className="muted small">Everything is stored on this Mac{info ? ` in ${info.dataDir}` : ''}. Nothing is sent anywhere.</p>
        <div className="row wrap">
          <button className="button" onClick={() => void run(api.openDataFolder())}>
            Show in Finder
          </button>
          <button className="button" onClick={() => void backupDatabase()}>
            Back up now…
          </button>
          <button className="button" onClick={() => void restoreDatabase()}>
            Restore from backup…
          </button>
        </div>
      </section>
      <section className="settings">
        <h3>Developer</h3>
        <div className="row wrap">
          <button className="button" onClick={() => (close(), void generateBenchmark())}>
            Generate benchmark board
          </button>
        </div>
        <p className="muted small">Milano Pen {info?.version ?? ''} · MIT License · no telemetry</p>
      </section>
    </Modal>
  );
}
