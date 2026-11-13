import '@fontsource-variable/geist';
import '@fontsource-variable/geist-mono';
import '@fontsource/instrument-serif/400-italic.css';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';

// The webview's own right-click menu (reload, inspect, print) has no place in the app.
// It stays in typing fields, so paste with the mouse still works.
window.addEventListener('contextmenu', (e) => {
  if (!(e.target as Element).closest?.('input, textarea')) e.preventDefault();
});

export default mount(App, { target: document.getElementById('app')! });
