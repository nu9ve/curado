import { tr } from '../lib/language';
import { useEffect, useRef, useState } from 'react';
import { Radio, X } from 'lucide-react';
import { call, native } from '../lib/bridge';

// Serialize mount, layout, tab switches and teardown, including React StrictMode remounts.
let operations: Promise<unknown> = Promise.resolve();
function enqueue<T>(operation: () => Promise<T>): Promise<T> {
  const next = operations.then(operation, operation);
  operations = next.catch(() => {});
  return next;
}
type Service = 'fractos' | 'soundcloud';
export function WebExplorer({
  onError,
  suspended,
}: {
  onError: (e: unknown) => void;
  suspended: boolean;
}) {
  const [service, setService] = useState<Service | null>('fractos');
  const [failed, setFailed] = useState(false);
  const [retry, setRetry] = useState(0);
  const host = useRef<HTMLDivElement>(null);
  const isSuspended = useRef(suspended);
  isSuspended.current = suspended;
  const refresh = useRef<() => void>(() => {});
  useEffect(() => {
    if (!native || !service || !host.current) return;
    let disposed = false;
    let frame = 0;
    let last = '';
    const element = host.current;
    setFailed(false);
    const update = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const rect = element.getBoundingClientRect();
        const bounds = {
          x: Math.max(0, rect.x),
          y: Math.max(0, rect.y),
          width: Math.max(1, rect.width),
          height: Math.max(1, rect.height),
          visible: !isSuspended.current && rect.width > 0 && rect.height > 0,
        };
        const key = JSON.stringify(bounds);
        if (key === last) return;
        last = key;
        void enqueue(async () => {
          if (!disposed) await call('open_web_service', { service, bounds });
        }).catch((error) => {
          if (!disposed) {
            setFailed(true);
            onError(error);
          }
        });
      });
    };
    refresh.current = update;
    const observer = new ResizeObserver(update);
    observer.observe(element);
    // Footer expansion, viewport resizing and scrolling all affect native child bounds.
    window.addEventListener('resize', update);
    window.addEventListener('scroll', update, true);
    update();
    return () => {
      disposed = true;
      cancelAnimationFrame(frame);
      observer.disconnect();
      window.removeEventListener('resize', update);
      window.removeEventListener('scroll', update, true);
      refresh.current = () => {};
      void enqueue(() => call('close_web_services')).catch(onError);
    };
  }, [service, retry, onError]);
  useEffect(() => {
    refresh.current();
  }, [suspended]);

  return (
    <section
      className="web-explorer"
      aria-label={tr('Servicios web y radio', 'Web services and radio')}
    >
      <div className="web-explorer-heading">
        <h2>radio / web</h2>
        <span className="web-audio-badge">
          {tr('audio web independiente', 'independent web audio')}
        </span>
      </div>
      <div
        className="web-explorer-actions"
        role="group"
        aria-label={tr('Servicio web', 'Web service')}
      >
        <button
          disabled={!native}
          aria-pressed={service === 'fractos'}
          onClick={() => setService('fractos')}
          data-help="web-fractos"
        >
          <Radio size={15} /> fractos radio
        </button>
        <button
          disabled={!native}
          aria-pressed={service === 'soundcloud'}
          onClick={() => setService('soundcloud')}
          data-help="web-soundcloud"
        >
          SoundCloud
        </button>
        <button
          disabled={!native || !service}
          className="subtle"
          onClick={() => setService(null)}
          data-help="web-close"
        >
          <X size={14} /> {tr('detener audio web', 'stop web audio')}{' '}
        </button>
        {failed && (
          <button onClick={() => setRetry((value) => value + 1)}>
            {tr('Reintentar', 'Retry')}
          </button>
        )}
      </div>
      <p className="web-audio-note">
        {tr(
          'El audio web se controla en su player. Master, FX, sync y Escape actúan sobre los decks locales. Cambiar de servicio o salir de esta vista detiene la reproducción web.',
          'Web audio is controlled in its player. Master, FX, sync and Escape affect local decks. Switching services or leaving this view stops web playback.',
        )}{' '}
      </p>
      <div
        ref={host}
        className="web-explorer-frame"
        aria-label={tr('Área del navegador integrado', 'Integrated browser area')}
      >
        {!native ? (
          <p>
            {tr(
              'El navegador integrado requiere curado de escritorio.',
              'The integrated browser requires the desktop app.',
            )}
          </p>
        ) : !service ? (
          <p>
            {tr(
              'Audio web detenido. Elige un servicio para volver a escuchar.',
              'Web audio stopped. Choose a service to listen again.',
            )}
          </p>
        ) : failed ? (
          <p role="alert">
            {tr(
              'No se pudo abrir el navegador. Puedes reintentar.',
              'Could not open the browser. You can retry.',
            )}
          </p>
        ) : (
          <span className="sr-only">
            {tr('Contenido de', 'Content from')}{' '}
            {service === 'fractos' ? 'fractos radio' : 'SoundCloud'}{' '}
            {tr('en un navegador integrado.', 'in an integrated browser.')}{' '}
          </span>
        )}
      </div>
    </section>
  );
}
