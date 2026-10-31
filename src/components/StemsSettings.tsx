import { useCallback, useEffect, useState } from 'react';
import { call, native } from '../lib/bridge';
import { tr } from '../lib/language';
import type { StemsStatus } from '../lib/types';

const gigabytes = (bytes: number) => `${(bytes / 1e9).toFixed(1)} GB`;

/** Where stems live, how much room they may take and which engine makes them. */
export function StemsSettings({ onError }: { onError: (e: unknown) => void }) {
  const [status, setStatus] = useState<StemsStatus | null>(null);
  const [limit, setLimit] = useState('');
  const refresh = useCallback(() => {
    if (!native) return;
    call<StemsStatus>('stems_status')
      .then((next) => {
        setStatus(next);
        setLimit(String(next.limitGb));
      })
      .catch(onError);
  }, [onError]);
  useEffect(refresh, [refresh]);
  if (!status) return null;
  return (
    <div className="stems-settings">
      <h3>stems</h3>
      <div className="info-grid">
        <span>
          {tr('Motor', 'Engine')}
          <b>
            {status.runtime
              ? `${status.runtime.engine} ${status.runtime.version}`
              : tr('No instalado', 'Not installed')}
          </b>
        </span>
        <span>
          {tr('Ocupan', 'In use')}
          <b>{gigabytes(status.usedBytes)}</b>
        </span>
        <span>
          {tr('Libre en el disco', 'Free on the disk')}
          <b>{status.freeBytes === null ? '—' : gigabytes(status.freeBytes)}</b>
        </span>
      </div>
      {!status.runtime && (
        <p className="small-print">
          {tr(
            'Opcional. Instálalo con scripts/setup-separation.sh, o elige un Python que ya tenga Demucs. Nada de esto va dentro de la app.',
            'Optional. Install it with scripts/setup-separation.sh, or choose a Python that already has Demucs. None of this goes inside the app.',
          )}
        </p>
      )}
      <p className="small-print" title={status.folder}>
        {status.folder}
      </p>
      <div className="settings-row">
        <button onClick={() => call('stems_choose_python').then(refresh).catch(onError)}>
          {tr('Elegir Python', 'Choose Python')}
        </button>
        <button onClick={() => call('stems_choose_folder').then(refresh).catch(onError)}>
          {tr('Elegir carpeta', 'Choose folder')}
        </button>
        <label>
          {tr('Límite (GB)', 'Limit (GB)')}{' '}
          <input
            type="number"
            min="1"
            max="4096"
            value={limit}
            onChange={(e) => setLimit(e.target.value)}
            onBlur={() =>
              call('stems_set_limit', { gigabytes: Math.round(+limit) })
                .then(refresh)
                .catch(onError)
            }
          />
        </label>
      </div>
      <p className="small-print">
        {tr(
          'Al pasar el límite se borran primero los stems más viejos que no marcaste: los que tienen favorito, etiqueta, playlist, banco o loop se quedan. Se pueden volver a generar.',
          'Past the limit, the oldest stems you did not mark go first: those with a favorite, tag, playlist, bank or loop stay. They can be made again.',
        )}
      </p>
    </div>
  );
}
