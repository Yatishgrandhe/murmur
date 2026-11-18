/* Murmur settings window */
const T = window.__TAURI__;
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];
const esc = s => String(s ?? '').replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

// ───────── bridge (with a mock so the UI can be previewed in a browser) ─────────
const mock = (() => {
  const s = { shortcut: 'Alt+Space', provider: 'claude', style: 'clean', custom_prompt: '', model: 'base.en', language: 'en', claude_model: 'haiku', codex_model: '', anthropic_model: 'claude-haiku-5-5', openai_model: 'gpt-5-mini', dictionary: ['Kubernetes', 'Murmur'], restore_clipboard: true, sounds: true, onboarded: !location.hash.includes('onboard') };
  const now = Date.now();
  const history = [
    { ts: now - 6e4, raw: 'um so can you send me the the report by friday no sorry thursday thanks', text: 'Can you send me the report by Thursday? Thanks!', provider: 'claude', seconds: 5.2, ms: 1180 },
    { ts: now - 36e5, raw: 'remind the team that standup moves to ten thirty tomorrow', text: 'Reminder: standup moves to 10:30 tomorrow.', provider: 'claude', seconds: 3.1, ms: 1040 },
    { ts: now - 9e7, raw: 'ok sounds good', text: 'Ok, sounds good', provider: 'off', seconds: 1.1, ms: 310 },
  ];
  return {
    get_state: () => ({ settings: s, history, platform: 'macos', vibrancy: false, defaultShortcut: 'Alt+Space', launchAtLogin: false, version: '0.1.0',
      models: [
        { id: 'tiny.en', label: 'Swift', blurb: 'Fastest · English', size_mb: 75, english_only: true, installed: false },
        { id: 'base.en', label: 'Balanced', blurb: 'Recommended · English', size_mb: 142, english_only: true, installed: true },
        { id: 'small.en', label: 'Precise', blurb: 'Most accurate · English', size_mb: 466, english_only: true, installed: false },
        { id: 'large-v3-turbo-q5_0', label: 'Worldwide', blurb: 'Best quality · 99 languages', size_mb: 547, english_only: false, installed: false },
      ] }),
    provider_status: () => ({ claude: { installed: true, logged_in: true, account: 'you@example.com', detail: 'Signed in with your Claude plan' }, codex: { installed: false, logged_in: false, detail: 'Not installed' }, anthropicKey: false, openaiKey: false, accessibility: false }),
    test_rewrite: async () => { await new Promise(r => setTimeout(r, 900)); return { text: "Let's push the deadline to Monday and let the team know ASAP. Thanks!", ms: 1120 }; },
  };
})();
const invoke = (cmd, args) => T ? T.core.invoke(cmd, args) : Promise.resolve(mock[cmd]?.(args));
const listen = (ev, fn) => T ? T.event.listen(ev, e => fn(e.payload)) : null;

let st = null;      // app state
let ps = null;      // provider status
const isMac = () => (st?.platform || 'macos') === 'macos';

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
  claude: '<svg viewBox="0 0 24 24" fill="white"><path d="M12 2.5l1.6 6.1 5.6-3-3 5.6 6.1 1.6-6.1 1.6 3 5.6-5.6-3L12 21.5l-1.6-6.1-5.6 3 3-5.6L1.7 11.2l6.1-1.6-3-5.6 5.6 3z"/></svg>',
  gpt: '<svg viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="12" cy="12" r="7.5"/><circle cx="12" cy="12" r="3"/></svg>',
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
  anthropic: { name: 'Claude API', sub: 'Pay-as-you-go key', color: 'linear-gradient(135deg,#8a7cff,#5b46e8)', icon: 'apikey' },
  openai: { name: 'OpenAI API', sub: 'Pay-as-you-go key', color: 'linear-gradient(135deg,#4a4a55,#26262d)', icon: 'apikey' },
  off: { name: 'No rewrite', sub: 'Raw transcript, instant', color: 'linear-gradient(135deg,#9a9aa3,#6c6c75)', icon: 'off' },
};
function provState(id) {
  if (!ps) return '';
  if (id === 'claude' || id === 'codex') return ps[id].logged_in ? 'ok' : ps[id].installed ? 'half' : '';
  if (id === 'anthropic') return ps.anthropicKey ? 'ok' : '';
  if (id === 'openai') return ps.openaiKey ? 'ok' : '';
  return 'ok';
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
  if (s.logged_in) {
    const models = id === 'claude'
      ? `<div class="sub-seg"><span class="muted">Model</span><div class="seg" id="cmodel">${['haiku', 'sonnet'].map(m => `<button data-v="${m}" class="${st.settings.claude_model === m ? 'on' : ''}">${m === 'haiku' ? 'Haiku · fastest' : 'Sonnet · smartest'}</button>`).join('')}</div></div>`
      : `<p class="muted" style="margin:12px 0 0">ChatGPT plans run Codex's full model, so rewrites take about 3 seconds. Claude Haiku takes about 1.</p><div class="keyrow"><input class="field" id="cxmodel" placeholder="Model (leave empty for your Codex default)" value="${esc(st.settings.codex_model)}"></div>`;
    return `<div class="pd"><div class="logo prov-logo" style="background:${p.color};width:40px;height:40px;border-radius:11px;display:grid;place-items:center">${ICON[p.icon]}</div>
      <div class="grow"><b>Connected to ${p.name}</b><p>${esc(s.detail)}${s.account ? ' · ' + esc(s.account) : ''}. Rewrites count toward your plan's normal usage.</p></div>
      <span class="badge ok">Ready</span></div>${models}`;
  }
  return `<div class="pd"><div class="grow"><b>Connect your ${p.name} plan</b>
      <p>Murmur talks to ${p.name} through the official ${tool} app, signed in with your own account. Your login stays with ${tool}.</p></div>
      <button class="btn ghost" id="recheck">Check again</button></div>
    <ol class="steps">
      <li class="${s.installed ? 'done' : ''}">Install ${tool}${s.installed ? '' : `<button class="btn primary" data-setup="install">Install</button>`}</li>
      <li class="${s.logged_in ? 'done' : ''}">Sign in with your ${p.name} account<button class="btn ${s.installed ? 'primary' : ''}" data-setup="login" ${s.installed ? '' : 'disabled'}>Sign in</button></li>
    </ol>`;
}
function renderDetail() {
  const id = st.settings.provider, el = $('#providerDetail');
  if (id === 'claude' || id === 'codex') el.innerHTML = cliDetail(id);
  else if (id === 'anthropic' || id === 'openai') {
    const has = id === 'anthropic' ? ps?.anthropicKey : ps?.openaiKey;
    const mkey = id === 'anthropic' ? 'anthropic_model' : 'openai_model';
    el.innerHTML = `<div class="pd"><div class="grow"><b>${PROV[id].name} key</b><p>Stored in your system keychain, never in a file. Billed by ${id === 'anthropic' ? 'Anthropic' : 'OpenAI'} per use.</p></div>${has ? '<span class="badge ok">Saved</span>' : ''}</div>
      <div class="keyrow"><input type="password" class="field" id="apikey" placeholder="${has ? '•••••••••••••••• (saved, type to replace)' : id === 'anthropic' ? 'sk-ant-…' : 'sk-…'}"><button class="btn primary" id="saveKey">Save</button></div>
      <div class="keyrow"><input class="field" id="apimodel" value="${esc(st.settings[mkey])}" placeholder="Model"></div>`;
    $('#saveKey').onclick = async () => {
      try { await invoke('set_api_key', { name: id + '_key', value: $('#apikey').value }); toast('Key saved to keychain'); await refreshProviders(); } catch (e) { toast(String(e)); }
    };
    $('#apimodel').onchange = e => save({ [mkey]: e.target.value.trim() });
  } else {
    el.innerHTML = `<div class="pd"><div class="grow"><b>Raw transcript</b><p>Murmur pastes exactly what the on-device model heard: no AI, no network, near-instant.</p></div></div>`;
  }
  $$('[data-setup]', el).forEach(b => b.onclick = async () => {
    try { const cmd = await invoke('setup_provider', { provider: id, action: b.dataset.setup }); toast(`Opened a terminal: ${cmd}`); pollProviders(); }
    catch (e) { toast(String(e)); }
  });
  const re = $('#recheck', el); if (re) re.onclick = () => refreshProviders(true);
  $$('#cmodel button', el).forEach(b => b.onclick = async () => { await save({ claude_model: b.dataset.v }); renderDetail(); });
  const cx = $('#cxmodel', el); if (cx) cx.onchange = e => save({ codex_model: e.target.value.trim() });
}
async function refreshProviders(announce) {
  ps = await invoke('provider_status');
  renderProviders(); renderStatus(); renderPerm();
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
  custom: 'Describe exactly how you want your text to read.',
};
function renderStyle() {
  $$('#styles button').forEach(b => b.classList.toggle('on', b.dataset.v === st.settings.style));
  $('#styleDesc').textContent = STYLE_DESC[st.settings.style] || '';
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
$('#autostart').onchange = async e => { try { await invoke('set_launch_at_login', { enabled: e.target.checked }); } catch (x) { toast(String(x)); e.target.checked = !e.target.checked; } };
$('#permBtn').onclick = () => invoke('open_privacy', { which: 'accessibility' });
function renderPerm() { $('#permCard').classList.toggle('hidden', !isMac() || !ps || ps.accessibility); }

// ───────── voice models ─────────
const dl = {};   // id -> progress 0..1
function renderModels() {
  $('#models').innerHTML = st.models.map(m => {
    const on = st.settings.model === m.id, p = dl[m.id];
    const action = p != null ? `<button class="btn" disabled>Downloading ${Math.round(p * 100)}%</button>`
      : m.installed ? (on ? `<span class="badge ok">In use</span>` : `<button class="btn primary" data-use="${m.id}">Use</button><button class="btn icon" data-rm="${m.id}" title="Delete"><i data-i="trash"></i></button>`)
      : `<button class="btn" data-dl="${m.id}">Download</button>`;
    return `<div class="model ${on && m.installed ? 'on' : ''}"><span class="size">${m.size_mb} MB</span><b>${m.label}</b><p>${m.blurb}</p><div class="row-btns">${action}</div>${p != null ? `<div class="bar" style="width:${p * 100}%"></div>` : ''}</div>`;
  }).join('');
  paintIcons($('#models'));
  $$('[data-dl]').forEach(b => b.onclick = () => download(b.dataset.dl));
  $$('[data-use]').forEach(b => b.onclick = async () => { await save({ model: b.dataset.use }); renderModels(); renderStatus(); });
  $$('[data-rm]').forEach(b => b.onclick = async () => { await invoke('delete_model', { id: b.dataset.rm }); st = await invoke('get_state'); renderModels(); });
  $('#language').value = st.settings.language;
}
async function download(id, use = true) {
  dl[id] = 0; renderModels(); renderOb();
  try {
    await invoke('download_model', { id });
    delete dl[id];
    st.models.find(m => m.id === id).installed = true;
    if (use) await save({ model: id });
    toast('Voice model ready');
  } catch (e) { delete dl[id]; toast(String(e)); }
  renderModels(); renderStatus(); renderOb();
}
listen('download', p => { dl[p.id] = p.total ? p.done / p.total : 0; if (p.done < p.total) { renderModels(); updateRing(); } });
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
  const model = st.models.find(m => m.id === st.settings.model);
  if (phase === 'listening') { s.classList.add('live'); t.textContent = 'Listening…'; return; }
  if (phase === 'transcribing' || phase === 'rewriting') { s.classList.add('live'); t.textContent = phase === 'rewriting' ? 'Polishing…' : 'Transcribing…'; return; }
  if (!model?.installed) { s.classList.add('warn'); t.textContent = 'Download a voice model'; return; }
  if (ps && provState(st.settings.provider) !== 'ok') { s.classList.add('warn'); t.textContent = `${PROV[st.settings.provider].name} not connected`; return; }
  if (ps && isMac() && !ps.accessibility) { s.classList.add('warn'); t.innerHTML = `Ready · <span class="keys sm">${keysHTML(st.settings.shortcut)}</span> · results copied, ⌘V`; return; }
  t.innerHTML = `Ready · <span class="keys sm">${keysHTML(st.settings.shortcut)}</span>`;
}
let heroEnergy = 0.25;
listen('phase', p => {
  renderStatus(p.phase);
  heroEnergy = p.phase === 'listening' ? 1 : (p.phase === 'transcribing' || p.phase === 'rewriting') ? .55 : .25;
  if (p.phase === 'done' && obStep === 4) { obDone = true; renderOb(); }
  if (p.phase === 'done' || p.phase === 'error' || p.phase === 'idle') setTimeout(() => renderStatus(), 1200);
});
listen('history-changed', async () => { const s = await invoke('get_state'); st.history = s.history; renderHome(); renderHistory(); });
listen('navigate', s => go(s));
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
let obStep = 0, obDone = false;
const OB_STEPS = () => isMac() ? 5 : 4;   // permissions step is macOS only
function obIndex(i) { return !isMac() && i >= 3 ? i + 1 : i; }
function updateRing() {
  const p = dl['base.en']; const c = $('#obRing'); if (!c || p == null) return;
  c.style.strokeDashoffset = 289 * (1 - p); $('#obPct').textContent = `${Math.round(p * 100)}%`;
}
function renderObIfAI() { if (!$('#onboard').classList.contains('hidden') && obIndex(obStep) === 2) renderOb(); }
function renderOb() {
  const ob = $('#onboard'); if (ob.classList.contains('hidden')) return;
  $('#obDots').innerHTML = Array.from({ length: OB_STEPS() }, (_, i) => `<i class="${i === obStep ? 'on' : ''}"></i>`).join('');
  const b = $('#obBody'), step = obIndex(obStep);
  const next = (label = 'Continue', extra = '') => `<div class="ob-actions">${extra}<button class="btn primary" id="obNext">${label}</button></div>`;
  if (step === 0) {
    b.innerHTML = `<h1>Meet Murmur</h1><p>Talk in any app. Murmur turns your voice into clean, well-written text in about a second.</p>${next('Get started')}`;
  } else if (step === 1) {
    const m = st.models.find(x => x.id === 'base.en'), p = dl['base.en'], ready = st.models.some(x => x.installed);
    b.innerHTML = `<h1>${ready ? 'Voice model ready' : 'Download the voice model'}</h1>
      <p>${ready ? 'Speech-to-text runs privately on this computer.' : `A one-time ${m.size_mb} MB download. Your voice is turned into text on this computer and never uploaded.`}</p>
      ${p != null ? `<div class="ring"><svg viewBox="0 0 100 100"><circle cx="50" cy="50" r="46" fill="none" stroke="rgba(255,255,255,.12)" stroke-width="6"/><circle id="obRing" cx="50" cy="50" r="46" fill="none" stroke="url(#rg)" stroke-width="6" stroke-linecap="round" stroke-dasharray="289" stroke-dashoffset="289" style="transition:stroke-dashoffset .2s"/><defs><linearGradient id="rg"><stop offset="0" stop-color="rgb(80,220,255)"/><stop offset=".5" stop-color="rgb(150,110,255)"/><stop offset="1" stop-color="rgb(255,90,190)"/></linearGradient></defs></svg><div class="pct" id="obPct">0%</div></div>` : ''}
      ${ready ? next() : p != null ? '' : next(`Download · ${m.size_mb} MB`)}`;
    updateRing();
  } else if (step === 2) {
    const opt = (id, extra) => { const p = PROV[id], s = provState(id); return `<div class="ob-opt ${st.settings.provider === id ? 'on' : ''}" data-p="${id}"><div class="logo" style="background:${p.color}">${ICON[p.icon]}</div><div><b>${p.name}</b><span>${extra}</span></div><span class="st ${s === 'ok' && id !== 'off' ? 'ok' : ''}">${id === 'off' ? 'No AI' : s === 'ok' ? 'Connected' : s === 'half' ? 'Sign in' : 'Set up'}</span></div>`; };
    const sel = st.settings.provider, s = ps?.[sel];
    let act = '';
    if ((sel === 'claude' || sel === 'codex') && s && !s.logged_in) act = s.installed ? `<button class="btn" data-setup="login">Sign in to ${PROV[sel].name}</button>` : `<button class="btn" data-setup="install">Install ${sel === 'claude' ? 'Claude Code' : 'Codex'}</button>`;
    b.innerHTML = `<h1>Connect your AI</h1><p>Use the plan you already have. Murmur signs in through the official app, so there's nothing new to pay for.</p>
      <div class="ob-opts">${opt('claude', 'Claude Pro or Max')}${opt('codex', 'ChatGPT Plus or Pro')}${opt('off', 'Skip, paste raw transcripts')}</div>
      ${next(provState(sel) === 'ok' ? 'Continue' : 'Skip for now', act)}`;
    $$('.ob-opt', b).forEach(o => o.onclick = async () => { await save({ provider: o.dataset.p }); renderOb(); renderProviders(); });
    $$('[data-setup]', b).forEach(x => x.onclick = async () => { try { await invoke('setup_provider', { provider: sel, action: x.dataset.setup }); pollProviders(); toast('Finish in the terminal window, then come back'); } catch (e) { toast(String(e)); } });
  } else if (step === 3) {
    const ok = ps?.accessibility;
    b.innerHTML = `<h1>${ok ? 'Paste access granted' : 'Allow pasting'}</h1>
      <p>${ok ? 'Murmur can type into any app.' : 'macOS asks you to allow Murmur under Accessibility so it can paste text where your cursor is. The first time you dictate, macOS will also ask for the microphone.'}</p>
      ${next(ok ? 'Continue' : 'Skip for now', ok ? '' : '<button class="btn" id="obPerm">Open Accessibility Settings</button>')}`;
    const pb = $('#obPerm'); if (pb) pb.onclick = () => { invoke('open_privacy', { which: 'accessibility' }); pollPerm(); };
  } else {
    b.innerHTML = `<h1>${obDone ? 'Beautiful.' : 'Give it a try'}</h1>
      <p>${obDone ? "That's it. Murmur lives in your menu bar and works in every app." : 'Click the box, hold the shortcut, say something, and let go.'}</p>
      <div class="keys ob-big-keys">${keysHTML(st.settings.shortcut)}</div>
      <textarea class="ob-try" id="obTry" placeholder="Your words will appear here…"></textarea>
      ${next(obDone ? 'Start using Murmur' : 'Finish')}`;
    setTimeout(() => $('#obTry')?.focus(), 300);
  }
  const n = $('#obNext');
  if (n) n.onclick = async () => {
    if (step === 1 && !st.models.some(x => x.installed)) return download('base.en');
    if (obStep >= OB_STEPS() - 1) { await save({ onboarded: true }); ob.classList.add('hidden'); renderAll(); return; }
    obStep++; renderOb();
  };
}
function pollPerm() {
  let n = 0; const t = setInterval(async () => { await refreshProviders(); if (ps.accessibility || ++n > 60) { clearInterval(t); renderOb(); } }, 2000);
}

// ───────── boot ─────────
function renderAll() {
  paintShortcut(); renderHome(); renderProviders(); renderStyle(); renderModels(); renderChips(); renderHistory(); renderStatus(); renderPerm();
  $('#sounds').checked = st.settings.sounds; $('#restore').checked = st.settings.restore_clipboard; $('#autostart').checked = st.launchAtLogin;
}
(async () => {
  st = await invoke('get_state');
  if (st.vibrancy) document.documentElement.classList.add('vibrant');
  if (st.platform === 'windows') document.documentElement.classList.add('win');
  $('#ver').textContent = 'v' + st.version;
  paintIcons();
  renderAll();
  waves($('#heroWave'), () => heroEnergy, .27);
  if (!st.settings.onboarded) { $('#onboard').classList.remove('hidden'); waves($('#obWave'), () => .6); renderOb(); }
  await refreshProviders(); renderOb();
  const r = $('.app').getBoundingClientRect();
  invoke('ui_report', { page: 'main', data: {
    onboardingVisible: !$('#onboard').classList.contains('hidden'), onboardingTitle: $('#obBody h1')?.textContent,
    providerCards: $$('#providers .prov').length, modelCards: $$('#models .model').length, navItems: $$('#nav a').length,
    status: $('#statusText').innerText, layout: [Math.round(r.width), Math.round(r.height)], vibrancy: st.vibrancy,
    dark: matchMedia('(prefers-color-scheme: dark)').matches, claude: ps?.claude?.detail, codex: ps?.codex?.detail, accessibility: ps?.accessibility,
  } }).catch(() => {});
})();
window.addEventListener('error', e => invoke('ui_report', { page: 'main', data: { error: String(e.message) } }).catch(() => {}));
if (T) T.event.listen('phase', e => invoke('ui_report', { page: 'main', data: { phaseSeen: e.payload.phase, status: $('#statusText').innerText } }).catch(() => {}));
