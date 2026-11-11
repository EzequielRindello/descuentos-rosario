// Applies the saved light/dark choice before the page paints (loaded without defer, so there is
// no flash of the wrong theme). No saved choice: the site follows the device. See setupTheme in app.js.
'use strict';
try {
  const theme = localStorage.getItem('ofertas-rosario:tema');
  if (theme === 'light' || theme === 'dark') document.documentElement.dataset.theme = theme;
} catch {
  // Storage blocked: follow the device.
}
