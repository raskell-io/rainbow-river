// Host for rainbow-river.wasm: input in, pixels out, 60 updates per second.

const W = 240, H = 160, STEP = 1000 / 60;
const SAVE_KEY = 'rainbow-river.sram';

const BTN = { right: 1, left: 2, up: 4, down: 8, a: 16, b: 32, select: 64, start: 128 };
const KEYS = {
  ArrowRight: BTN.right, ArrowLeft: BTN.left, ArrowUp: BTN.up, ArrowDown: BTN.down,
  KeyD: BTN.right, KeyA: BTN.left, KeyW: BTN.up, KeyS: BTN.down,
  KeyX: BTN.a, KeyK: BTN.a, KeyZ: BTN.b, KeyJ: BTN.b,
  Enter: BTN.start, ShiftLeft: BTN.select, ShiftRight: BTN.select,
};

const canvas = document.getElementById('screen');
const ctx = canvas.getContext('2d');
const image = ctx.createImageData(W, H);

const { instance } = await WebAssembly.instantiateStreaming(fetch('rainbow-river.wasm'));
const k = instance.exports;
k.rr_init((Math.random() * 2 ** 32) >>> 0);
loadSave();

let keys = 0;
addEventListener('keydown', (e) => {
  if (e.code in KEYS) { keys |= KEYS[e.code]; e.preventDefault(); }
});
addEventListener('keyup', (e) => { if (e.code in KEYS) keys &= ~KEYS[e.code]; });
addEventListener('blur', () => { keys = 0; });

function gamepad() {
  let b = 0;
  for (const pad of navigator.getGamepads?.() ?? []) {
    if (!pad) continue;
    const on = (i) => pad.buttons[i]?.pressed;
    const [x = 0, y = 0] = pad.axes;
    if (on(0)) b |= BTN.a;
    if (on(1)) b |= BTN.b;
    if (on(8)) b |= BTN.select;
    if (on(9)) b |= BTN.start;
    if (on(12) || y < -0.5) b |= BTN.up;
    if (on(13) || y > 0.5) b |= BTN.down;
    if (on(14) || x < -0.5) b |= BTN.left;
    if (on(15) || x > 0.5) b |= BTN.right;
  }
  return b;
}

function loadSave() {
  try {
    const saved = localStorage.getItem(SAVE_KEY);
    if (!saved) return;
    const bytes = Uint8Array.from(atob(saved), (c) => c.charCodeAt(0));
    new Uint8Array(k.memory.buffer, k.rr_sram(), k.rr_sram_len()).set(bytes.subarray(0, k.rr_sram_len()));
    k.rr_sram_loaded();
  } catch { /* no storage: play without saves */ }
}

function persist() {
  if (!k.rr_sram_take_dirty()) return;
  try {
    const bytes = new Uint8Array(k.memory.buffer, k.rr_sram(), k.rr_sram_len());
    localStorage.setItem(SAVE_KEY, btoa(String.fromCharCode(...bytes)));
  } catch { /* ignore */ }
}

function fit() {
  const scale = Math.max(1, Math.floor(Math.min(innerWidth / W, (innerHeight - 60) / H)));
  canvas.style.width = `${W * scale}px`;
  canvas.style.height = `${H * scale}px`;
}
addEventListener('resize', fit);
fit();

// Fixed timestep: the game runs at 60 Hz whatever the display refresh rate.
let last = performance.now(), acc = 0;
function tick(now) {
  acc = Math.min(acc + now - last, STEP * 5);
  last = now;
  let ran = false;
  while (acc >= STEP) {
    k.rr_frame(keys | gamepad());
    acc -= STEP;
    ran = true;
  }
  if (ran) {
    image.data.set(new Uint8Array(k.memory.buffer, k.rr_framebuffer(), W * H * 4));
    ctx.putImageData(image, 0, 0);
    persist();
  }
  requestAnimationFrame(tick);
}
requestAnimationFrame(tick);
