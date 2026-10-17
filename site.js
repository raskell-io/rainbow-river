import { createConsole, KEY } from './play/wasm-ad.js';

const $ = (sel) => document.querySelector(sel);
const handheld = $('#handheld');
const dock = $('#play');
const nowPlaying = $('#now-playing');
const CART = 'carts/rainbow-river.wasm';

// The console peeks out of the hero until the visitor clicks into it; then it
// rises and takes keyboard input. Until then arrow keys keep scrolling the page.
let engaged = false;
function engage(on) {
  engaged = on;
  handheld.classList.toggle('engaged', on);
  dock.classList.toggle('raised', on);
}
function bringIntoView() {
  dock.scrollIntoView({ behavior: 'smooth', block: 'end' });
}
document.addEventListener('pointerdown', (e) => {
  const wasEngaged = engaged;
  engage(Boolean(e.target.closest('#handheld, #play-cta')));
  if (!wasEngaged && e.target.closest('#handheld')) setTimeout(bringIntoView, 350);
});
$('#play-cta').addEventListener('click', (e) => {
  e.preventDefault();
  engage(true);
  setTimeout(bringIntoView, 350);
});
$('#engage').addEventListener('click', () => engage(true));

const ad = await createConsole({
  canvas: $('#screen'),
  core: new URL('./play/core.wasm', import.meta.url),
  active: () => engaged,
  onError: (e) => { nowPlaying.innerHTML = `<span class="err">${e.message}</span>`; },
});

// On-screen buttons, multi-touch: each pointer holds one button.
const held = new Map();
function syncVirtual() {
  let mask = 0;
  for (const btn of held.values()) mask |= KEY[btn.dataset.key];
  ad.setVirtualKeys(mask);
  for (const b of handheld.querySelectorAll('[data-key]')) b.classList.toggle('pressed', [...held.values()].includes(b));
}
handheld.addEventListener('pointerdown', (e) => {
  const btn = e.target.closest('[data-key]');
  if (!btn) return;
  e.preventDefault();
  btn.setPointerCapture?.(e.pointerId);
  held.set(e.pointerId, btn);
  syncVirtual();
});
for (const type of ['pointerup', 'pointercancel', 'lostpointercapture']) {
  handheld.addEventListener(type, (e) => { if (held.delete(e.pointerId)) syncVirtual(); });
}
const KEYCODES = { ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right', KeyX: 'a', KeyZ: 'b', KeyA: 'l', KeyS: 'r', Enter: 'start', ShiftLeft: 'select', ShiftRight: 'select' };
for (const [type, on] of [['keydown', true], ['keyup', false]]) {
  addEventListener(type, (e) => {
    const k = KEYCODES[e.code];
    if (k && (engaged || !on)) handheld.querySelector(`[data-key="${k}"]`)?.classList.toggle('pressed', on);
  });
}

try {
  const res = await fetch(CART);
  if (!res.ok) throw new Error(`could not load the cart (${res.status})`);
  const meta = await ad.load(await res.arrayBuffer());
  nowPlaying.textContent = `Now playing: ${meta.title}${meta.version ? ` v${meta.version}` : ''}`;
} catch (e) {
  nowPlaying.innerHTML = `<span class="err">${e.message}</span>`;
}
