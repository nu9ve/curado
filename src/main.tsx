import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';
document.documentElement.dataset.platform = navigator.userAgent.includes('Mac')
  ? 'macos'
  : navigator.userAgent.includes('Windows')
    ? 'windows'
    : 'linux';
class Boundary extends React.Component<{ children: React.ReactNode }, { error: string | null }> {
  state = { error: null as string | null };
  static getDerivedStateFromError(error: Error) {
    return { error: error.message };
  }
  render() {
    return this.state.error ? (
      <div className="fatal">
        <h1>La interfaz necesita reiniciarse.</h1>
        <p>El audio puede seguir activo. Usa el menú de Curado para silenciarlo.</p>
        <pre>{this.state.error}</pre>
        <button onClick={() => location.reload()}>Recargar interfaz</button>
      </div>
    ) : (
      this.props.children
    );
  }
}
ReactDOM.createRoot(document.getElementById('root')!).render(
  <Boundary>
    <App />
  </Boundary>,
);
