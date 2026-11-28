/* Murmur settings window */
const T = window.__TAURI__;
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];
const esc = s => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

// ───────── bridge (with a mock so the UI can be previewed in a browser) ─────────
const mock = (() => {
  const s = { shortcut: 'Alt+Space', provider: 'claude', style: 'clean', custom_prompt: '', language: 'en', claude_model: 'haiku', codex_model: '', anthropic_model: 'claude-haiku-5-5', openai_model: 'gpt-5-mini', speech_engine: 'openai', speech_model: 'gpt-transcribe', dictionary: ['Kubernetes', 'Murmur'], restore_clipboard: true, sounds: true, onboarded: !location.hash.includes('onboard') };
  const now = Date.now();
  const history = [
    { ts: now - 6e4, raw: 'um so can you send me the the report by friday no sorry thursday thanks', text: 'Can you send me the report by Thursday? Thanks!', provider: 'claude', seconds: 5.2, ms: 1180 },
    { ts: now - 36e5, raw: 'remind the team that standup moves to ten thirty tomorrow', text: 'Reminder: standup moves to 10:30 tomorrow.', provider: 'claude', seconds: 3.1, ms: 1040 },
    { ts: now - 9e7, raw: 'ok sounds good', text: 'Ok, sounds good', provider: 'off', seconds: 1.1, ms: 310 },
  ];
  return {
    get_state: () => ({ settings: s, history, platform: 'macos', vibrancy: false, defaultShortcut: 'Alt+Space', launchAtLogin: false, version: '0.4.0',
      speech: { engine: 'apple', ready: true, can_request: false, on_device: true, detail: 'Apple speech recognition, on this Mac' },
      systemSpeech: { engine: 'apple', ready: true, can_request: false, on_device: true, detail: 'Apple speech recognition, on this Mac' } }),
    permissions: () => ({ mic: 'granted', speech: 'granted', accessibility: 'not_asked', all_granted: false, not_installed: false }),
    request_permission: () => ({ mic: 'granted', speech: 'granted', accessibility: 'granted', all_granted: true, not_installed: false }),
    verify_provider: async () => { await new Promise(r => setTimeout(r, 1200)); return { ok: true, ms: 1100 }; },
    find_installers: () => [],
    provider_status: () => ({ verified: { claude: true }, claude: { installed: true, logged_in: true, account: 'you@example.com', detail: 'Signed in with your Claude plan' }, codex: { installed: false, logged_in: false, detail: 'Not installed' }, anthropicKey: false, openaiKey: false, accessibility: false }),
    speech_status: () => ({ engine: 'apple', ready: true, can_request: false, on_device: true, detail: 'Apple speech recognition, on this Mac' }),
    request_speech: () => ({ engine: 'apple', ready: true, can_request: false, on_device: true, detail: 'Apple speech recognition, on this Mac' }),
    test_rewrite: async () => { await new Promise(r => setTimeout(r, 900)); return { text: "Let's push the deadline to Monday and let the team know ASAP. Thanks!", ms: 1120 }; },
  };
})();
const invoke = (cmd, args) => T ? T.core.invoke(cmd, args) : Promise.resolve(mock[cmd]?.(args));
const listen = (ev, fn) => T ? T.event.listen(ev, e => fn(e.payload)) : null;

let st = null;      // app state
let ps = null;      // provider status
const isMac = () => (st?.platform || 'macos') === 'macos';
const pick = (mac, win) => (isMac() ? mac : win);   // platform wording for inline strings
// elements with data-win="..." get that text on Windows; the HTML text stays as the macOS copy
function paintPlatform(root = document) { $$('[data-win]', root).forEach(e => { if (!isMac()) e.textContent = e.dataset.win; }); }

// ───────── icons ─────────
const ICON = {
  home: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/></svg>',
  spark: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M12 2c.6 4.9 2.6 8.5 9 10-6.4 1.5-8.4 5.1-9 10-.6-4.9-2.6-8.5-9-10 6.4-1.5 8.4-5.1 9-10Z"/></svg>',
  key: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><rect x="2.5" y="6" width="19" height="12" rx="2.5"/><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10"/></svg>',
  wave: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round"><path d="M3 10v4M7 7v10M11 4v16M15 8v8M19 6v12M23 11v2"/></svg>',
  book: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4.5A1.5 1.5 0 0 1 5.5 3H20v15H5.5A1.5 1.5 0 0 0 4 19.5zM4 19.5A1.5 1.5 0 0 0 5.5 21H20"/></svg>',
  clock: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></svg>',
  copy: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="8" y="8" width="12" height="12" rx="2.5"/><path d="M16 8V5.5A1.5 1.5 0 0 0 14.5 4h-9A1.5 1.5 0 0 0 4 5.5v9A1.5 1.5 0 0 0 5.5 16H8"/></svg>',
  trash: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/></svg>',
  claude: '<svg viewBox="0 0 24 24" fill="white"><path d="m4.7144 15.9555 4.7174-2.6471.079-.2307-.079-.1275h-.2307l-.7893-.0486-2.6956-.0729-2.3375-.0971-2.2646-.1214-.5707-.1215-.5343-.7042.0546-.3522.4797-.3218.686.0608 1.5179.1032 2.2767.1578 1.6514.0972 2.4468.255h.3886l.0546-.1579-.1336-.0971-.1032-.0972L6.973 9.8356l-2.55-1.6879-1.3356-.9714-.7225-.4918-.3643-.4614-.1578-1.0078.6557-.7225.8803.0607.2246.0607.8925.686 1.9064 1.4754 2.4893 1.8336.3643.3035.1457-.1032.0182-.0728-.164-.2733-1.3539-2.4467-1.445-2.4893-.6435-1.032-.17-.6194c-.0607-.255-.1032-.4674-.1032-.7285L6.287.1335 6.6997 0l.9957.1336.419.3642.6192 1.4147 1.0018 2.2282 1.5543 3.0296.4553.8985.2429.8318.091.255h.1579v-.1457l.1275-1.706.2368-2.0947.2307-2.6957.0789-.7589.3764-.9107.7468-.4918.5828.2793.4797.686-.0668.4433-.2853 1.8517-.5586 2.9021-.3643 1.9429h.2125l.2429-.2429.9835-1.3053 1.6514-2.0643.7286-.8196.85-.9046.5464-.4311h1.0321l.759 1.1293-.34 1.1657-1.0625 1.3478-.8804 1.1414-1.2628 1.7-.7893 1.36.0729.1093.1882-.0183 2.8535-.607 1.5421-.2794 1.8396-.3157.8318.3886.091.3946-.3278.8075-1.967.4857-2.3072.4614-3.4364.8136-.0425.0304.0486.0607 1.5482.1457.6618.0364h1.621l3.0175.2247.7892.522.4736.6376-.079.4857-1.2142.6193-1.6393-.3886-3.825-.9107-1.3113-.3279h-.1822v.1093l1.0929 1.0686 2.0035 1.8092 2.5075 2.3314.1275.5768-.3218.4554-.34-.0486-2.2039-1.6575-.85-.7468-1.9246-1.621h-.1275v.17l.4432.6496 2.3436 3.5214.1214 1.0807-.17.3521-.6071.2125-.6679-.1214-1.3721-1.9246L14.38 17.959l-1.1414-1.9428-.1397.079-.674 7.2552-.3156.3703-.7286.2793-.6071-.4614-.3218-.7468.3218-1.4753.3886-1.9246.3157-1.53.2853-1.9004.17-.6314-.0121-.0425-.1397.0182-1.4328 1.9672-2.1796 2.9446-1.7243 1.8456-.4128.164-.7164-.3704.0667-.6618.4008-.5889 2.386-3.0357 1.4389-1.882.929-1.0868-.0062-.1579h-.0546l-6.3385 4.1164-1.1293.1457-.4857-.4554.0608-.7467.2307-.2429 1.9064-1.3114Z"/></svg>',
  gpt: '<svg viewBox="0 0 24 24" fill="white"><path d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.872zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997Z"/></svg>',
  anthropic: '<svg viewBox="0 0 24 24" fill="white"><path d="M17.3041 3.541h-3.6718l6.696 16.918H24Zm-10.6082 0L0 20.459h3.7442l1.3693-3.5527h7.0052l1.3693 3.5528h3.7442L10.5363 3.5409Zm-.3712 10.2232 2.2914-5.9456 2.2914 5.9456Z"/></svg>',
  apikey: '<svg viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round"><circle cx="8" cy="15" r="4"/><path d="m11 12 9-9M17 6l3 3M15 8l2 2"/></svg>',
  off: '<svg viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round"><path d="M4 7h16M4 12h10M4 17h13"/></svg>',
};
const paintIcons = (root = document) => $$('i[data-i]', root).forEach(i => { i.innerHTML = ICON[i.dataset.i] || ''; });

// ───────── helpers ─────────
function toast(msg) {
  const t = $('#toast'); t.textContent = msg; t.classList.add('on');
  clearTimeout(toast.t); toast.t = setTimeout(() => t.classList.remove('on'), 2400);
}
async function save(patch) {
  const prev = st.settings;
  st.settings = { ...st.settings, ...patch };
  try { await invoke('save_settings', { new: st.settings }); return true; }
  catch (e) { st.settings = prev; toast(String(e)); return false; }
}
function keyLabel(k) {
  const mac = isMac();
  const m = { Alt: mac ? '⌥' : 'Alt', Option: '⌥', Shift: mac ? '⇧' : 'Shift', Ctrl: mac ? '⌃' : 'Ctrl', Control: mac ? '⌃' : 'Ctrl',
    Super: mac ? '⌘' : 'Win', Cmd: '⌘', Command: '⌘', Meta: mac ? '⌘' : 'Win', CommandOrControl: mac ? '⌘' : 'Ctrl', CmdOrCtrl: mac ? '⌘' : 'Ctrl',
    Space: 'Space', Backquote: '`', Minus: '-', Equal: '=', Comma: ',', Period: '.', Slash: '/', Backslash: '\\', Semicolon: ';', Quote: "'", BracketLeft: '[', BracketRight: ']' };
  return m[k] ?? k;
}
const keysHTML = accel => accel.split('+').map(k => `<kbd>${esc(keyLabel(k))}</kbd>`).join('');
function paintShortcut() { $$('[data-shortcut]').forEach(e => { e.innerHTML = keysHTML(st.settings.shortcut); }); }
function ago(ts) {
  const s = (Date.now() - ts) / 1000;
  if (s < 60) return 'just now'; if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`; return new Date(ts).toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}
const words = t => (t.match(/\S+/g) || []).length;
const PROV = {
  claude: { name: 'Claude', sub: 'Your Pro or Max plan', color: 'linear-gradient(135deg,#e08a64,#c2603d)', icon: 'claude' },
  codex: { name: 'ChatGPT', sub: 'Your Plus or Pro plan', color: 'linear-gradient(135deg,#2bc29b,#0f8f6e)', icon: 'gpt' },
  anthropic: { name: 'Claude API', sub: 'Pay-as-you-go key', color: 'linear-gradient(135deg,#d9a07c,#a35b3a)', icon: 'anthropic' },
  openai: { name: 'OpenAI API', sub: 'Pay-as-you-go key', color: 'linear-gradient(135deg,#4a4a55,#1c1c22)', icon: 'gpt' },
  off: { name: 'No rewrite', sub: 'Raw transcript, instant', color: 'linear-gradient(135deg,#9a9aa3,#6c6c75)', icon: 'off' },
};
// "ok" only after a real test message went through the user's account (or a dictation did);
// signed in but unverified is "half".
function provState(id) {
  if (!ps) return '';
  if (id === 'off') return 'ok';
  if (ps.verified?.[id]) return 'ok';
  if (id === 'claude' || id === 'codex') return ps[id].installed ? 'half' : '';
  return (id === 'anthropic' ? ps.anthropicKey : ps.openaiKey) ? 'half' : '';
}
const signedIn = id => !!ps && (id === 'claude' || id === 'codex' ? ps[id].logged_in : id === 'anthropic' ? ps.anthropicKey : id === 'openai' ? ps.openaiKey : true);
const verifying = {}, verifyErr = {};
async function verifyProvider(id, quiet) {
  if (verifying[id]) return;
  verifying[id] = true; delete verifyErr[id]; renderDetail(); renderOb();
  try {
    const r = await invoke('verify_provider', { provider: id });
    if (!r.ok) verifyErr[id] = r.error || 'The test message failed.';
    else if (!quiet) toast(`${PROV[id].name} connected`);
  } catch (e) { verifyErr[id] = String(e); }
  verifying[id] = false;
  await refreshProviders();
}
// Signed in but never verified: check once automatically (one short test message).
function autoVerify() {
  const id = st.settings.provider;
  if (id === 'off' || !ps || ps.verified?.[id] || verifying[id] || verifyErr[id] || !signedIn(id)) return;
  verifyProvider(id, true);
}

// ───────── navigation ─────────
function go(s) {
  $$('#nav a').forEach(a => a.classList.toggle('on', a.dataset.s === s));
  $$('main section').forEach(x => x.classList.toggle('on', x.dataset.s === s));
  $('#main').scrollTop = 0;
  if (s === 'ai' || s === 'shortcut') refreshProviders();
}
$$('#nav a').forEach(a => a.onclick = () => go(a.dataset.s));
document.addEventListener('click', e => { const g = e.target.closest('[data-go]'); if (g) go(g.dataset.go); });

// ───────── home ─────────
function renderHome() {
  const h = st.history;
  const w = h.reduce((n, i) => n + words(i.text), 0);
  $('#stWords').textContent = w.toLocaleString();
  const savedMin = Math.max(0, w / 40 - w / 150);   // typing ~40 wpm vs speaking ~150 wpm
  $('#stSaved').textContent = savedMin >= 60 ? `${(savedMin / 60).toFixed(1)}h` : `${Math.round(savedMin)}m`;
  const recent = h.filter(i => i.provider !== 'off').slice(0, 20);
  $('#stSpeed').textContent = recent.length ? `${(recent.reduce((n, i) => n + i.ms, 0) / recent.length / 1000).toFixed(1)}s` : '–';
  $('#recent').innerHTML = h.length ? h.slice(0, 4).map(itemHTML).join('') : `<div class="empty"><b>Nothing yet</b>Your dictations will show up here.</div>`;
  paintIcons($('#recent'));
}
function itemHTML(i) {
  const changed = i.raw && i.raw.trim() !== i.text.trim();
  return `<div class="item" data-ts="${i.ts}">
    <p>${esc(i.text)}</p>
    ${changed ? `<p class="raw">${esc(i.raw)}</p>` : ''}
    <div class="meta"><span>${ago(i.ts)}</span><span class="badge ${i.provider === 'off' ? '' : 'ok'}">${esc(PROV[i.provider]?.name === 'No rewrite' ? 'Raw' : PROV[i.provider]?.name || i.provider)}</span><span>${i.seconds.toFixed(1)}s audio · ${(i.ms / 1000).toFixed(1)}s</span></div>
    <div class="acts"><button class="btn icon" data-copy title="Copy"><i data-i="copy"></i></button><button class="btn icon" data-del title="Delete"><i data-i="trash"></i></button></div>
  </div>`;
}
document.addEventListener('click', async e => {
  const item = e.target.closest('.item'); if (!item) return;
  const ts = Number(item.dataset.ts), it = st.history.find(i => i.ts === ts);
  if (e.target.closest('[data-copy]')) { await invoke('copy_text', { text: it.text }); toast('Copied'); }
  if (e.target.closest('[data-del]')) { await invoke('delete_history_item', { ts }); st.history = st.history.filter(i => i.ts !== ts); renderHome(); renderHistory(); }
});

// ───────── AI ─────────
function renderProviders() {
  $('#providers').innerHTML = Object.entries(PROV).map(([id, p]) => `
    <button class="prov ${st.settings.provider === id ? 'on' : ''}" data-p="${id}">
      <span class="st ${provState(id)}"></span>
      <div class="logo" style="background:${p.color}">${ICON[p.icon]}</div>
      <b>${p.name}</b><span>${p.sub}</span>
    </button>`).join('');
  $$('#providers .prov').forEach(b => b.onclick = async () => { if (await save({ provider: b.dataset.p })) { renderProviders(); renderDetail(); renderStatus(); } });
  renderDetail();
}
function cliDetail(id) {
  const s = ps?.[id] || {}; const p = PROV[id];
  const tool = id === 'claude' ? 'Claude Code' : 'Codex';
  if (s.logged_in && !ps.verified?.[id]) {
    const busy = verifying[id], err = verifyErr[id];
    return `<div class="pd"><div class="logo prov-logo" style="background:${p.color};width:40px;height:40px;border-radius:11px;display:grid;place-items:center">${ICON[p.icon]}</div>
      <div class="grow"><b>${busy ? `Checking your ${p.name} connection…` : err ? `Couldn't reach ${p.name}` : 'Signed in, not verified yet'}</b>
      <p>${busy ? 'Sending one short test message through your account.' : err ? esc(err) : `Murmur sends one short test message through your ${p.name} account to make sure it works.`}</p></div>
      ${busy ? '<span class="badge">Checking…</span>' : `<button class="btn primary" id="verifyBtn">${err ? 'Try again' : 'Verify'}</button>`}</div>
      ${err ? `<ol class="steps"><li class="done">Install ${tool}</li><li>Sign in again with your ${p.name} account<button class="btn" data-setup="login">Sign in</button></li></ol>` : ''}`;
  }
  if (s.logged_in) {
    const models = id === 'claude'
      ? `<div class="sub-seg"><span class="muted">Model</span><div class="seg" id="cmodel">${['haiku', 'sonnet'].map(m => `<button data-v="${m}" class="${st.settings.claude_model === m ? 'on' : ''}">${m === 'haiku' ? 'Haiku · fastest' : 'Sonnet · smartest'}</button>`).join('')}</div></div>`
      : `<p class="muted" style="margin:12px 0 0">ChatGPT plans run Codex's full model, so rewrites take about 3 seconds. Claude Haiku takes about 1.</p><div class="keyrow"><input class="field" id="cxmodel" placeholder="Model (leave empty for your Codex default)" value="${esc(st.settings.codex_model)}"></div>`;
    return `<div class="pd"><div class="logo prov-logo" style="background:${p.color};width:40px;height:40px;border-radius:11px;display:grid;place-items:center">${ICON[p.icon]}</div>
      <div class="grow"><b>Connected to ${p.name}</b><p>${esc(s.detail)}${s.account ? ' · ' + esc(s.account) : ''}. Verified with a test message. Rewrites count toward your plan's normal usage.</p></div>
      <span class="badge ok">Ready</span></div>${models}`;
  }
  return `<div class="pd"><div class="grow"><b>Connect your ${p.name} plan</b>
      <p>Murmur talks to ${p.name} through the official ${tool} app, signed in with your own account. Your login stays with ${tool}.</p></div>
      <button class="btn ghost" id="recheck">Check again</button></div>
    <ol class="steps">
      <li class="${s.installed ? 'done' : ''}">Install ${tool}${s.installed ? '' : `<button class="btn primary" data-setup="install">Install</button>`}</li>
      <li class="${s.logged_in ? 'done' : ''}">Sign in with your ${p.name} account<button class="btn ${s.installed ? 'primary' : ''}" data-setup="login" ${s.installed ? '' : 'disabled'}>Sign in</button></li>
      <li>Murmur verifies the connection with one short test message</li>
    </ol>
    <p class="muted" style="margin:10px 0 0">Sign in opens a terminal running ${tool}'s own login. You sign in on ${id === 'claude' ? 'claude.ai' : 'chatgpt.com'}; Murmur never sees your password.</p>`;
}
function renderDetail() {
  const id = st.settings.provider, el = $('#providerDetail');
  if (id === 'claude' || id === 'codex') el.innerHTML = cliDetail(id);
  else if (id === 'anthropic' || id === 'openai') {
    const has = id === 'anthropic' ? ps?.anthropicKey : ps?.openaiKey;
    const mkey = id === 'anthropic' ? 'anthropic_model' : 'openai_model';
    const ver = ps?.verified?.[id], busy = verifying[id], err = verifyErr[id];
    const badge = !has ? '' : ver ? '<span class="badge ok">Verified</span>' : busy ? '<span class="badge">Checking…</span>' : `<button class="btn" id="verifyBtn">${err ? 'Try again' : 'Verify'}</button>`;
    el.innerHTML = `<div class="pd"><div class="grow"><b>${PROV[id].name} key</b><p>${err ? esc(err) : `Stored in your system keychain, never in a file. Billed by ${id === 'anthropic' ? 'Anthropic' : 'OpenAI'} per use.`}</p></div>${badge}</div>
      <div class="keyrow"><input type="password" class="field" id="apikey" placeholder="${has ? '•••••••••••••••• (saved, type to replace)' : id === 'anthropic' ? 'sk-ant-…' : 'sk-…'}"><button class="btn primary" id="saveKey">Save</button></div>
      <div class="keyrow"><input class="field" id="apimodel" value="${esc(st.settings[mkey])}" placeholder="Model"></div>`;
    $('#saveKey').onclick = async () => {
      try { await invoke('set_api_key', { name: id + '_key', value: $('#apikey').value }); toast('Key saved to keychain'); await refreshProviders(); } catch (e) { toast(String(e)); }
    };
    $('#apimodel').onchange = e => save({ [mkey]: e.target.value.trim() });
  } else {
    el.innerHTML = `<div class="pd"><div class="grow"><b>Raw transcript</b><p>Murmur pastes exactly what speech recognition heard, with no AI rewrite. Near-instant.</p></div></div>`;
  }
  $$('[data-setup]', el).forEach(b => b.onclick = async () => {
    try { const cmd = await invoke('setup_provider', { provider: id, action: b.dataset.setup }); toast(`Opened a terminal: ${cmd}`); pollProviders(); }
    catch (e) { toast(String(e)); }
  });
  const re = $('#recheck', el); if (re) re.onclick = () => refreshProviders(true);
  const vb = $('#verifyBtn', el); if (vb) vb.onclick = () => { delete verifyErr[id]; verifyProvider(id); };
  $$('#cmodel button', el).forEach(b => b.onclick = async () => { await save({ claude_model: b.dataset.v }); renderDetail(); });
  const cx = $('#cxmodel', el); if (cx) cx.onchange = e => save({ codex_model: e.target.value.trim() });
}
async function refreshProviders(announce) {
  ps = await invoke('provider_status');
  renderProviders(); renderStatus(); renderPerm(); renderSpeech();
  autoVerify();
  if (announce) toast(provState(st.settings.provider) === 'ok' ? 'All set' : 'Not connected yet');
  return ps;
}
let pollT = null;
function pollProviders() {
  clearInterval(pollT); let n = 0;
  pollT = setInterval(async () => { const p = await refreshProviders(); if (++n > 60 || provState(st.settings.provider) === 'ok') { clearInterval(pollT); if (n <= 60) toast(`${PROV[st.settings.provider].name} connected`); } renderObIfAI(); }, 3000);
}

const STYLE_DESC = {
  clean: 'Fixes grammar and punctuation, drops the ums and false starts. Your words, just tidy.',
  polished: 'Clear, concise and professional. Tightens wording while keeping every point.',
  casual: 'Relaxed, like a text to a friend.',
  email: 'Turns what you say into a ready-to-send email.',
  prompt: 'Prompt mode: describe what you want from an AI, and Murmur writes a full, structured prompt with the goal, context, key requirements, constraints and output format. Also in the menu bar.',
  custom: 'Describe exactly how you want your text to read.',
};
function renderStyle() {
  $$('#styles button').forEach(b => b.classList.toggle('on', b.dataset.v === st.settings.style));
  $('#styleDesc').textContent = (STYLE_DESC[st.settings.style] || '').replace('menu bar', pick('menu bar', 'system tray'));
  $('#promptDemo').classList.toggle('hidden', st.settings.style !== 'prompt');
  $('#customPrompt').classList.toggle('hidden', st.settings.style !== 'custom');
  $('#customPrompt').value = st.settings.custom_prompt;
}
$$('#styles button').forEach(b => b.onclick = async () => { await save({ style: b.dataset.v }); renderStyle(); });
$('#customPrompt').onchange = e => save({ custom_prompt: e.target.value });
$('#testBtn').onclick = async () => {
  const btn = $('#testBtn'), out = $('#testOut');
  btn.disabled = true; btn.innerHTML = '<i data-i="spark"></i>Rewriting…'; paintIcons(btn);
  out.classList.add('hidden'); $('#testMs').textContent = '';
  try {
    const r = await invoke('test_rewrite', { text: $('#testIn').value, settings: st.settings });
    out.className = 'out'; out.textContent = r.text; $('#testMs').textContent = `${(r.ms / 1000).toFixed(2)}s`;
  } catch (e) { out.className = 'out bad'; out.textContent = String(e); }
  btn.disabled = false; btn.innerHTML = '<i data-i="spark"></i>Rewrite'; paintIcons(btn);
};

// ───────── shortcut ─────────
let recording = false;
$('#recorder').onclick = async () => {
  if (recording) return stopRec();
  recording = true; $('#recorder').classList.add('rec'); $('#recHint').textContent = 'Press your new shortcut… (esc to cancel)';
  $('#shortcutErr').classList.add('hidden');
  await invoke('pause_shortcut', { paused: true });
};
async function stopRec() {
  recording = false; $('#recorder').classList.remove('rec'); $('#recHint').textContent = 'Click to change';
  paintShortcut(); await invoke('pause_shortcut', { paused: false }).catch(e => showShortcutErr(e));
}
function showShortcutErr(e) { const el = $('#shortcutErr'); el.textContent = String(e); el.classList.remove('hidden'); }
window.addEventListener('keydown', async e => {
  if (!recording) return;
  e.preventDefault();
  if (e.key === 'Escape') return stopRec();
  const mods = [];
  if (e.ctrlKey) mods.push('Ctrl'); if (e.altKey) mods.push('Alt'); if (e.shiftKey) mods.push('Shift'); if (e.metaKey) mods.push('Super');
  let k = e.code;
  if (/^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/.test(k)) { $('#recKeys').innerHTML = keysHTML(mods.join('+') || '…'); return; }
  k = k.replace(/^Key/, '').replace(/^Digit/, '');
  if (!mods.length && !/^F\d+$/.test(k)) { showShortcutErr('Add a modifier like ' + (isMac() ? '⌥, ⌘, ⌃ or ⇧' : 'Ctrl, Alt or Shift') + '.'); return; }
  const accel = [...mods, k].join('+');
  recording = false; $('#recorder').classList.remove('rec'); $('#recHint').textContent = 'Click to change';
  if (await save({ shortcut: accel })) { paintShortcut(); toast('Shortcut updated'); }
  else { paintShortcut(); await invoke('pause_shortcut', { paused: false }).catch(() => {}); }
});
$('#resetShortcut').onclick = async () => { if (await save({ shortcut: st.defaultShortcut })) { paintShortcut(); toast('Shortcut reset'); } };
$('#sounds').onchange = e => save({ sounds: e.target.checked });
$('#restore').onchange = e => save({ restore_clipboard: e.target.checked });
$('#wakeWord').onchange = async e => { if (await save({ wake_word: e.target.checked })) { $('#wakeRow').classList.toggle('hidden', !e.target.checked); toast(e.target.checked ? `Say "${st.settings.wake_phrase}" to start` : 'Wake word off'); } };
$('#wakePhrase').onchange = e => { const v = e.target.value.trim(); if (v.split(/\s+/).length > 4) return toast('Keep it to 1–4 words'); save({ wake_phrase: v || 'Hey Murmur' }); };
$('#autostart').onchange = async e => { try { await invoke('set_launch_at_login', { enabled: e.target.checked }); } catch (x) { toast(String(x)); e.target.checked = !e.target.checked; } };
$('#permBtn').onclick = () => invoke('open_privacy', { which: 'accessibility' });
function renderPerm() { $('#permCard').classList.toggle('hidden', !isMac() || !ps || ps.accessibility); }

// ───────── voice (OpenAI speech-to-text, or the OS's built-in recognizer) ─────────
const speechOk = () => !!st.speech?.ready;
const cloudVoice = () => st.settings.speech_engine === 'openai';
function renderSpeech() {
  const sys = st.systemSpeech || st.speech || {}, mac = isMac(), cloud = cloudVoice(), hasKey = !!ps?.openaiKey;
  const osName = mac ? 'Apple Speech' : 'Windows Speech';
  const seg = `<div class="seg" id="engineSeg"><button data-e="openai" class="${cloud ? 'on' : ''}">OpenAI</button><button data-e="system" class="${cloud ? '' : 'on'}">Built-in (${mac ? 'Apple' : 'Windows'})</button></div>`;
  let body;
  if (cloud) {
    const model = st.settings.speech_model || 'gpt-transcribe';
    body = `<div class="pd"><div class="grow"><b>OpenAI ${esc(model)}</b><p>${hasKey
        ? 'Your recording is sent to OpenAI with your API key and transcribed there. Silence is trimmed first, and your Dictionary words are sent as spelling hints.'
        : `Add an OpenAI API key to use it. Until then Murmur uses ${osName}.`}</p></div>${hasKey ? '<span class="badge ok">Ready</span>' : '<span class="badge no">Needs key</span>'}</div>
      <div class="keyrow" style="margin-top:12px"><input id="voiceKey" class="field" type="password" placeholder="${hasKey ? 'Key saved in your keychain. Paste a new one to replace it' : 'sk-…  (platform.openai.com › API keys)'}"><button class="btn primary" id="voiceKeyBtn">Save key</button></div>
      <div class="keyrow" style="margin-top:8px"><select id="speechModel" class="field">
        ${[['gpt-transcribe', 'gpt-transcribe (recommended)'], ['gpt-4o-mini-transcribe', 'gpt-4o-mini-transcribe (cheapest)'], ['gpt-4o-transcribe', 'gpt-4o-transcribe'], ['whisper-1', 'whisper-1']]
          .map(([v, l]) => `<option value="${v}"${v === model ? ' selected' : ''}>${l}</option>`).join('')}
      </select></div>
      <p class="muted" style="margin:10px 0 0">Uses the same key as the <b>OpenAI API</b> option in AI. Claude has no speech-to-text API, so Claude (or ChatGPT) rewrites the transcript after OpenAI hears it.${mac ? ' If OpenAI can’t be reached, Murmur falls back to Apple Speech.' : ''} The wake word always stays on this computer.</p>`;
  } else {
    const where = mac
      ? (sys.on_device ? 'Runs on this Mac. Your audio stays here.' : "Uses Apple's speech recognition. On Macs without on-device support, audio is sent to Apple to be transcribed.")
      : "Uses Microsoft's online speech recognition, built into Windows. Audio is sent to Microsoft while you dictate.";
    const action = sys.ready ? '<span class="badge ok">Ready</span>'
      : `<button class="btn primary" id="speechBtn">${mac ? (sys.can_request ? 'Allow Speech Recognition' : 'Open Privacy Settings') : 'Open Speech Settings'}</button>`;
    body = `<div class="pd"><div class="grow"><b>${osName}</b><p>${esc(sys.ready ? where : sys.detail || '')}</p></div>${action}</div>
      <p class="muted" style="margin:12px 0 0">Built into ${mac ? 'macOS' : 'Windows'}, so there's nothing to download. Murmur's AI rewrite then fixes punctuation, filler words and mistakes.</p>
      ${!mac && !sys.ready ? '<p class="muted" style="margin:8px 0 0">In Settings → Privacy &amp; security → Speech, turn on <b>Online speech recognition</b>, then come back.</p>' : ''}`;
  }
  $('#models').innerHTML = `<div class="card speech" style="grid-column:1/-1">
    <div class="card-h"><h3>Speech recognition</h3>${seg}</div>${body}</div>`;
  $$('#engineSeg button').forEach(btn => btn.onclick = async () => {
    if (await save({ speech_engine: btn.dataset.e })) await refreshSpeech();
  });
  const b = $('#speechBtn'); if (b) b.onclick = requestSpeech;
  const kb = $('#voiceKeyBtn'); if (kb) kb.onclick = async () => {
    const v = $('#voiceKey').value.trim(); if (!v) return toast('Paste your OpenAI API key first');
    try { await invoke('set_api_key', { name: 'openai_key', value: v }); toast('Key saved to keychain'); await refreshProviders(); await refreshSpeech(); } catch (e) { toast(String(e)); }
  };
  const sm = $('#speechModel'); if (sm) sm.onchange = e => save({ speech_model: e.target.value }).then(refreshSpeech);
  $('#language').value = st.settings.language;
}
async function refreshSpeech() {
  const g = await invoke('get_state');
  st.speech = g.speech; st.systemSpeech = g.systemSpeech;
  renderSpeech(); renderStatus(); renderOb();
}
async function requestSpeech() {
  if (isMac() && !st.speech?.can_request) invoke('open_privacy', { which: 'speech' });
  st.speech = await invoke('request_speech');
  pollSpeech();
}
let speechTimer = null;
function pollSpeech() {
  clearInterval(speechTimer); let n = 0;
  const tick = async () => {
    st.speech = await invoke('speech_status');
    renderSpeech(); renderStatus(); renderOb();
    if (st.speech.ready || ++n > 60) clearInterval(speechTimer);
  };
  tick(); speechTimer = setInterval(tick, 2000);
}
$('#language').onchange = e => save({ language: e.target.value });

// ───────── dictionary ─────────
function renderChips() {
  $('#chips').innerHTML = st.settings.dictionary.map((w, i) => `<span class="chip">${esc(w)}<button data-x="${i}" title="Remove">×</button></span>`).join('') || '<span class="muted">No words yet.</span>';
  $$('#chips [data-x]').forEach(b => b.onclick = async () => { const d = [...st.settings.dictionary]; d.splice(+b.dataset.x, 1); await save({ dictionary: d }); renderChips(); });
}
$('#chipIn').onkeydown = async e => {
  if (e.key !== 'Enter') return;
  const vals = e.target.value.split(',').map(s => s.trim()).filter(Boolean).filter(v => !st.settings.dictionary.includes(v));
  if (!vals.length) return;
  await save({ dictionary: [...st.settings.dictionary, ...vals] }); e.target.value = ''; renderChips();
};

// ───────── history ─────────
function renderHistory() {
  $('#history').innerHTML = st.history.length ? st.history.map(itemHTML).join('') : `<div class="empty"><b>No history</b>Hold ${keysHTML(st.settings.shortcut)} and say something.</div>`;
  paintIcons($('#history'));
}
$('#clearHist').onclick = async () => { await invoke('clear_history'); st.history = []; renderHistory(); renderHome(); toast('History cleared'); };

// ───────── live status ─────────
function renderStatus(phase) {
  const s = $('#status'), t = $('#statusText');
  s.className = 'status';
  if (phase === 'listening') { s.classList.add('live'); t.textContent = 'Listening…'; return; }
  if (phase === 'transcribing' || phase === 'rewriting') { s.classList.add('live'); t.textContent = phase === 'rewriting' ? 'Polishing…' : 'Transcribing…'; return; }
  if (!speechOk()) { s.classList.add('warn'); t.textContent = isMac() ? 'Allow Speech Recognition' : 'Turn on Windows speech'; return; }
  if (ps && provState(st.settings.provider) !== 'ok') { s.classList.add('warn'); t.textContent = verifying[st.settings.provider] ? `Checking ${PROV[st.settings.provider].name}…` : `${PROV[st.settings.provider].name} not connected`; return; }
  if (ps && isMac() && !ps.accessibility) { s.classList.add('warn'); t.innerHTML = `Ready · <span class="keys sm">${keysHTML(st.settings.shortcut)}</span> · results copied, ⌘V`; return; }
  t.innerHTML = `Ready · <span class="keys sm">${keysHTML(st.settings.shortcut)}</span>`;
}
let heroEnergy = 0.25;
listen('phase', p => {
  renderStatus(p.phase);
  heroEnergy = p.phase === 'listening' ? 1 : (p.phase === 'transcribing' || p.phase === 'rewriting') ? .55 : .25;
  if (p.phase === 'done' && obIndex(obStep) === 4) { obDone = true; renderOb(); }
  if (p.phase === 'done' || p.phase === 'error' || p.phase === 'idle') setTimeout(() => renderStatus(), 1200);
});
listen('history-changed', async () => { const s = await invoke('get_state'); st.history = s.history; renderHome(); renderHistory(); });
listen('navigate', s => s === 'setup' ? openSetup() : go(s));
listen('settings-changed', async () => { const s = await invoke('get_state'); st.settings = s.settings; renderStyle(); });
window.addEventListener('focus', () => { if (st) refreshProviders(); });

// ───────── waves (hero + onboarding) ─────────
function waves(canvas, energyFn, cyFrac = .42) {
  const ctx = canvas.getContext('2d');
  const rib = [[80, 220, 255, 1.4, 1.2, 0], [150, 110, 255, 2.0, -.9, 1.6], [255, 90, 190, 1.1, .7, 3.2], [255, 255, 255, 2.6, -1.5, 4.1]];
  let e = .2;
  function draw(now) {
    const d = devicePixelRatio || 1, r = canvas.getBoundingClientRect();
    if (canvas.width !== Math.round(r.width * d)) { canvas.width = Math.round(r.width * d); canvas.height = Math.round(r.height * d); }
    const W = canvas.width, H = canvas.height, t = now / 1000;
    e += (energyFn() - e) * .05;
    ctx.clearRect(0, 0, W, H);
    ctx.globalCompositeOperation = 'lighter';
    const cy = H * cyFrac;
    for (const [R, G, B, f, s, p] of rib) {
      const amp = H * (.09 + .2 * e) * (R === 255 && G === 255 ? .45 : 1) * (1 + .25 * Math.sin(t * .9 + p));
      const grad = ctx.createLinearGradient(0, 0, W, 0);
      grad.addColorStop(0, `rgba(${R},${G},${B},0)`); grad.addColorStop(.5, `rgba(${R},${G},${B},.55)`); grad.addColorStop(1, `rgba(${R},${G},${B},0)`);
      ctx.beginPath();
      for (let i = 0; i <= 140; i++) {
        const u = i / 140, xn = (u - .5) * 4, att = Math.pow(4 / (4 + xn ** 4), 2);
        const y = cy + Math.sin(xn * f * Math.PI + t * s * (1 + e * 2) + p) * att * amp;
        i ? ctx.lineTo(u * W, y) : ctx.moveTo(u * W, y);
      }
      for (let i = 140; i >= 0; i--) {
        const u = i / 140, xn = (u - .5) * 4, att = Math.pow(4 / (4 + xn ** 4), 2);
        ctx.lineTo(u * W, cy - Math.sin(xn * f * Math.PI + t * s * (1 + e * 2) + p) * att * amp * .3);
      }
      ctx.closePath(); ctx.fillStyle = grad; ctx.shadowColor = `rgba(${R},${G},${B},.8)`; ctx.shadowBlur = 24 * d; ctx.fill();
    }
    ctx.globalCompositeOperation = 'source-over'; ctx.shadowBlur = 0;
    requestAnimationFrame(draw);
  }
  requestAnimationFrame(draw);
}

// ───────── onboarding ─────────
// Steps are worked out from what's actually missing, so anything already granted or connected
// (a reinstall, a second Mac with the same account) is skipped instead of asked again.
let perms = null, obPlan = ['try'], obStep = 0, obDone = false, permPoll = null;
function obNeeded({ welcome = true, tryIt = true } = {}) {
  const steps = [];
  if (welcome && !st.settings.onboarded && !st.history.length) steps.push('welcome');
  if (!perms?.all_granted) steps.push('perms');
  if (provState(st.settings.provider) !== 'ok') steps.push('ai');
  if (tryIt) steps.push('try');
  return steps;
}
function openOnboarding(plan) {
  obPlan = plan; obStep = 0; obDone = false;
  const ob = $('#onboard');
  if (ob.classList.contains('hidden')) { ob.classList.remove('hidden'); waves($('#obWave'), () => .6); }
  renderOb();
}
// A key press found something missing: open setup at the first missing step.
async function openSetup() {
  perms = await invoke('permissions'); await refreshProviders();
  const plan = obNeeded({ welcome: false, tryIt: false });
  if (plan.length) openOnboarding(plan); else go('voice');
}
function renderObIfAI() { if (!$('#onboard').classList.contains('hidden') && obPlan[obStep] === 'ai') renderOb(); }
function watchPerms(on) {
  clearInterval(permPoll);
  if (on) permPoll = setInterval(async () => { const before = JSON.stringify(perms); perms = await invoke('permissions'); if (JSON.stringify(perms) !== before) renderOb(); }, 1500);
}
const PERM_INFO = () => isMac() ? [
  ['mic', 'Microphone', 'So Murmur can hear you, only while you hold the shortcut (or while the wake word is on, if you turn it on).'],
  ['speech', 'Speech Recognition', "Apple's built-in speech-to-text turns your voice into words, on this Mac when it can."],
  ['accessibility', 'Accessibility', "Lets Murmur press ⌘V to paste where your cursor is. Optional: without it, text is copied and you paste it yourself."],
] : [
  ['mic', 'Microphone', 'Settings › Privacy & security › Microphone: turn on "Let desktop apps access your microphone".'],
  ['speech', 'Online speech recognition', "Settings › Privacy & security › Speech. Windows' built-in dictation; audio is sent to Microsoft while you dictate."],
];
function renderOb() {
  const ob = $('#onboard'); if (ob.classList.contains('hidden')) return;
  const step = obPlan[obStep] || 'try';
  watchPerms(step === 'perms');
  $('#obDots').innerHTML = obPlan.length > 1 ? obPlan.map((_, i) => `<i class="${i === obStep ? 'on' : ''}"></i>`).join('') : '';
  const b = $('#obBody');
  const next = (label = 'Continue', extra = '') => `<div class="ob-actions">${extra}<button class="btn primary" id="obNext">${label}</button></div>`;
  if (step === 'welcome') {
    b.innerHTML = `<h1>Meet Murmur</h1><p>Talk in any app. Murmur turns your voice into clean, well-written text in about a second. Setup takes a minute: ${isMac() ? 'three' : 'two'} permissions and your AI account.</p>${next('Get started')}`;
  } else if (step === 'perms') {
    const pm = perms || {}, good = v => v === 'granted' || v === 'unknown';
    const rows = PERM_INFO().map(([k, name, why]) => {
      const v = pm[k], done = good(v);
      const label = done ? '' : isMac() && v === 'not_asked' ? 'Allow' : 'Open Settings';
      return `<li class="${done ? 'done' : ''}"><div class="pt"><b>${name}</b><span>${why}</span></div>${done ? '<span class="st ok">On</span>' : `<button class="btn" data-perm="${k}">${label}</button>`}</li>`;
    }).join('');
    const axHelp = isMac() && !good(pm.accessibility) && pm.accessibility_tried
      ? `<p class="ob-note">Already switched on in System Settings but still not working? That's an old entry from a previous version. <a id="axReset">Reset Murmur's entry</a> and allow it again.</p>` : '';
    const dmg = pm.not_installed ? `<p class="ob-note warn">Murmur is running from the disk image. Drag it into Applications and open it from there, or macOS won't remember these permissions.</p>` : '';
    const required = good(pm.mic) && good(pm.speech);
    b.innerHTML = `<h1>${pm.all_granted ? 'All set' : 'A few permissions'}</h1>
      <p>Murmur asks for each one only when you click its button, and ${isMac() ? 'macOS remembers your answer' : 'Windows remembers these settings'}.</p>
      ${dmg}<ul class="ob-perms">${rows}</ul>${axHelp}
      ${next(required ? 'Continue' : 'Skip for now')}`;
    $$('[data-perm]', b).forEach(x => x.onclick = async () => {
      x.disabled = true;
      if (x.dataset.perm === 'accessibility') perms = { ...perms, accessibility_tried: true };
      const keep = perms?.accessibility_tried;
      perms = { ...(await invoke('request_permission', { which: x.dataset.perm })), accessibility_tried: keep };
      renderOb(); renderSpeech(); renderPerm();
    });
    const ar = $('#axReset'); if (ar) ar.onclick = async () => {
      if (!confirm("Remove Murmur's old Accessibility entry? macOS will then ask you to allow Murmur again.")) return;
      try { perms = await invoke('reset_accessibility'); toast('Now allow Murmur in the window macOS shows'); } catch (e) { toast(String(e)); }
      renderOb();
    };
  } else if (step === 'ai') {
    const sel = st.settings.provider, s = ps?.[sel];
    const opt = (id, extra) => { const p = PROV[id], st2 = provState(id); return `<div class="ob-opt ${sel === id ? 'on' : ''}" data-p="${id}"><div class="logo" style="background:${p.color}">${ICON[p.icon]}</div><div><b>${p.name}</b><span>${extra}</span></div><span class="st ${st2 === 'ok' && id !== 'off' ? 'ok' : ''}">${id === 'off' ? 'No AI' : st2 === 'ok' ? 'Connected' : verifying[id] ? 'Checking…' : ps?.[id]?.logged_in ? 'Verify' : ps?.[id]?.installed ? 'Sign in' : 'Set up'}</span></div>`; };
    let steps = '';
    if ((sel === 'claude' || sel === 'codex') && s) {
      const tool = sel === 'claude' ? 'Claude Code' : 'Codex', site = sel === 'claude' ? 'claude.ai' : 'chatgpt.com', ver = ps.verified?.[sel];
      steps = `<ol class="steps ob-steps-list">
        <li class="${s.installed ? 'done' : ''}">Install ${tool}, the official app Murmur talks through${s.installed ? '' : '<button class="btn" data-setup="install">Install</button>'}</li>
        <li class="${s.logged_in && !verifyErr[sel] ? 'done' : ''}">Sign in on ${site} with your own account${!s.installed || (s.logged_in && !verifyErr[sel]) ? '' : `<button class="btn" data-setup="login">${s.logged_in ? 'Sign in again' : 'Sign in'}</button>`}</li>
        <li class="${ver ? 'done' : ''}">${verifying[sel] ? 'Checking the connection…' : verifyErr[sel] ? `Test failed: ${esc(verifyErr[sel])}` : 'Murmur checks the connection with one short test message'}${s.logged_in && !ver && !verifying[sel] ? `<button class="btn" id="obVerify">${verifyErr[sel] ? 'Try again' : 'Verify'}</button>` : ''}</li>
      </ol>`;
    }
    b.innerHTML = `<h1>Connect your AI</h1><p>Use the plan you already have. You sign in through ${sel === 'codex' ? 'Codex' : 'Claude Code'}'s own login in a terminal window; Murmur never sees your password.</p>
      <div class="ob-opts">${opt('claude', 'Claude Pro or Max')}${opt('codex', 'ChatGPT Plus or Pro')}${opt('off', 'Skip, paste raw transcripts')}</div>
      ${steps}
      ${next(provState(sel) === 'ok' ? 'Continue' : 'Skip for now')}`;
    $$('.ob-opt', b).forEach(o => o.onclick = async () => { await save({ provider: o.dataset.p }); renderOb(); renderProviders(); autoVerify(); });
    $$('[data-setup]', b).forEach(x => x.onclick = async () => { try { await invoke('setup_provider', { provider: sel, action: x.dataset.setup }); pollProviders(); toast('Finish in the terminal window, then come back'); } catch (e) { toast(String(e)); } });
    const vb = $('#obVerify'); if (vb) vb.onclick = () => { delete verifyErr[sel]; verifyProvider(sel); };
  } else {
    b.innerHTML = `<h1>${obDone ? 'Beautiful.' : 'Give it a try'}</h1>
      <p>${obDone ? `That's it. Murmur lives in your ${pick('menu bar', 'system tray')} and works in every app.` : 'Click the box, hold the shortcut, say something, and let go.'}</p>
      <div class="keys ob-big-keys">${keysHTML(st.settings.shortcut)}</div>
      <textarea class="ob-try" id="obTry" placeholder="Your words will appear here…"></textarea>
      ${next(obDone ? 'Start using Murmur' : 'Finish')}`;
    setTimeout(() => $('#obTry')?.focus(), 300);
  }
  const n = $('#obNext');
  if (n) n.onclick = async () => {
    if (obStep >= obPlan.length - 1) {
      watchPerms(false);
      if (obPlan.includes('try') || obNeeded({ welcome: false, tryIt: false }).length === 0) await save({ onboarded: true });
      ob.classList.add('hidden'); renderAll(); checkInstallers(); return;
    }
    obStep++; renderOb();
  };
}
function pollPerm() {
  let n = 0; const t = setInterval(async () => { await refreshProviders(); if (ps.accessibility || ++n > 60) { clearInterval(t); renderOb(); } }, 2000);
}

// ───────── installer clean-up (asks; never deletes on its own) ─────────
async function checkInstallers() {
  if (perms?.not_installed) return;
  const files = await invoke('find_installers').catch(() => []);
  if (!files?.length) return;
  const list = files.map(f => `<li><b>${esc(f.name)}</b><span>${f.size_mb.toFixed(1)} MB · Downloads</span></li>`).join('');
  const m = $('#modal');
  $('#modalBody').innerHTML = `<h3>Delete the installer?</h3>
    <p>Murmur is installed, so you don't need ${files.length > 1 ? 'these files' : 'this file'} any more. ${isMac() ? 'It goes to the Trash' : 'It goes to the Recycle Bin'}, so you can still get it back.</p>
    <ul class="files">${list}</ul>
    <div class="row-btns"><button class="btn ghost" id="instKeep">Keep</button><button class="btn primary" id="instTrash">Move to ${isMac() ? 'Trash' : 'Recycle Bin'}</button></div>`;
  m.classList.remove('hidden');
  const names = files.map(f => f.name);
  const close = () => m.classList.add('hidden');
  $('#instKeep').onclick = async () => { await invoke('handle_installers', { names, delete: false }).catch(() => {}); close(); };
  $('#instTrash').onclick = async () => {
    try { const n = await invoke('handle_installers', { names, delete: true }); toast(`Moved ${n} file${n === 1 ? '' : 's'} to the ${isMac() ? 'Trash' : 'Recycle Bin'}`); }
    catch (e) { toast(String(e)); }
    close();
  };
}

// ───────── boot ─────────
function renderAll() {
  paintShortcut(); renderHome(); renderProviders(); renderStyle(); renderSpeech(); renderChips(); renderHistory(); renderStatus(); renderPerm();
  $('#sounds').checked = st.settings.sounds; $('#restore').checked = st.settings.restore_clipboard; $('#autostart').checked = st.launchAtLogin;
  $('#wakeWord').checked = !!st.settings.wake_word; $('#wakePhrase').value = st.settings.wake_phrase || 'Hey Murmur'; $('#wakeRow').classList.toggle('hidden', !st.settings.wake_word);
}
const bootStart = performance.now();
function dismissSplash() {
  const sp = $('#splash'); if (!sp) return;
  setTimeout(() => { sp.classList.add('out'); setTimeout(() => sp.remove(), 700); }, Math.max(0, 1100 - (performance.now() - bootStart)));
}
(async () => {
  st = await invoke('get_state');
  if (st.vibrancy) document.documentElement.classList.add('vibrant');
  if (st.platform === 'windows') document.documentElement.classList.add('win');
  paintPlatform();
  $('#ver').textContent = 'v' + st.version;
  paintIcons();
  renderAll();
  dismissSplash();
  waves($('#heroWave'), () => heroEnergy, .27);
  perms = await invoke('permissions').catch(() => null);
  await refreshProviders();
  if (!st.settings.onboarded) {
    const plan = obNeeded();
    // reinstall with everything still granted and connected: nothing to ask, don't repeat setup
    if (plan.length === 1 && plan[0] === 'try' && st.history.length) await save({ onboarded: true });
    else openOnboarding(plan);
  } else checkInstallers();
  const r = $('.app').getBoundingClientRect();
  invoke('ui_report', { page: 'main', data: {
    onboardingVisible: !$('#onboard').classList.contains('hidden'), onboardingTitle: $('#obBody h1')?.textContent, onboardingPlan: obPlan.join(','), perms, verified: ps?.verified,
    providerCards: $$('#providers .prov').length, speechCard: !!$('#models .speech'), navItems: $$('#nav a').length,
    status: $('#statusText').innerText, layout: [Math.round(r.width), Math.round(r.height)], vibrancy: st.vibrancy,
    dark: matchMedia('(prefers-color-scheme: dark)').matches, claude: ps?.claude?.detail, codex: ps?.codex?.detail, accessibility: ps?.accessibility,
  } }).catch(() => {});
})();
window.addEventListener('error', e => invoke('ui_report', { page: 'main', data: { error: String(e.message) } }).catch(() => {}));
window.addEventListener('unhandledrejection', e => invoke('ui_report', { page: 'main', data: { error: 'async: ' + String(e.reason?.message || e.reason) } }).catch(() => {}));
if (T) T.event.listen('phase', e => invoke('ui_report', { page: 'main', data: { phaseSeen: e.payload.phase, status: $('#statusText').innerText } }).catch(() => {}));
