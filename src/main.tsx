import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import './styles/tokens.css';
import './styles/fonts.css';
import './styles/app.css';

// macOS draws the window buttons over the top bar (overlay title bar).
document.documentElement.dataset.platform = /Mac/i.test(navigator.userAgent) ? 'mac' : 'other';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
