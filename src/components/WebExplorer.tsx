import { useState } from 'react';
import { ExternalLink, Radio, X } from 'lucide-react';
import { call, native } from '../lib/bridge';

export function WebExplorer({
  onError,
  onNotice,
}: {
  onError: (e: unknown) => void;
  onNotice: (text: string) => void;
}) {
  const [busy, setBusy] = useState(false);
  const open = async (service: 'fractos' | 'soundcloud') => {
    setBusy(true);
    try {
      await call('open_web_service', { service });
      onNotice(
        `Ventana de ${service === 'fractos' ? 'fractos radio' : 'SoundCloud'} abierta. Su audio se controla desde esa web.`,
      );
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  };
  const close = async () => {
    setBusy(true);
    try {
      const closed = await call<number>('close_web_services');
      onNotice(
        closed
          ? `Cierre solicitado para ${closed} ventana${closed === 1 ? '' : 's'} web.`
          : 'No hay ventanas web abiertas.',
      );
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="web-explorer" aria-label="Servicios web y radio">
      <div className="web-explorer-heading">
        <div>
          <span className="eyebrow">explorar</span>
          <h2>radio y servicios web</h2>
        </div>
        <span className="web-audio-badge">audio web independiente</span>
      </div>
      <p>
        Abre fractos radio o SoundCloud en sus propias ventanas y escucha mientras organizas tu
        biblioteca.
      </p>
      <div className="web-explorer-actions">
        <button
          disabled={!native || busy}
          onClick={() => void open('fractos')}
          data-help="web-fractos"
        >
          <Radio size={15} />
          fractos radio
          <ExternalLink size={12} />
        </button>
        <button
          disabled={!native || busy}
          onClick={() => void open('soundcloud')}
          data-help="web-soundcloud"
        >
          <ExternalLink size={15} />
          SoundCloud
        </button>
        <button
          disabled={!native || busy}
          className="subtle"
          onClick={() => void close()}
          data-help="web-close"
        >
          <X size={14} />
          detener audio web y cerrar
        </button>
      </div>
      <p className="web-audio-note">
        El master, crossfader, efectos, sync y Escape de curado controlan los decks locales. Para
        detener el audio web, pausa su player o cierra estas ventanas.
      </p>
      {!native && <small>La apertura de servicios requiere curado de escritorio.</small>}
      <details className="web-mixing-info">
        <summary>Sobre mezclar estos servicios</summary>
        <p>
          fractos reproduce con iframes de YouTube y SoundCloud. Sus players pueden ofrecer
          controles de reproducción y volumen, pero esta vista no los conecta al motor de audio. La
          mezcla web por volúmenes y la entrada de audio a los decks son integraciones distintas,
          todavía pendientes.
        </p>
      </details>
    </section>
  );
}
