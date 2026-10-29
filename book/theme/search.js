(() => {
  const main = document.querySelector('main');
  const buttons = document.querySelector('.left-buttons');
  if (!main || !buttons) return;
  const zh = document.documentElement.lang === 'zh-CN';
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'icon-button';
  button.id = 'ti-search-toggle';
  button.textContent = '⌕';
  button.title = zh ? '搜索本书 (/)' : 'Search this book (/)';
  button.setAttribute('aria-label', button.title);
  button.setAttribute('aria-expanded', 'false');
  button.setAttribute('aria-controls', 'ti-search');
  buttons.append(button);
  const panel = document.createElement('section');
  panel.id = 'ti-search'; panel.hidden = true; panel.className = 'ti-search';
  const label = document.createElement('label');
  label.htmlFor = 'ti-search-input'; label.textContent = zh ? '搜索本书' : 'Search this book';
  const input = document.createElement('input');
  input.id = 'ti-search-input'; input.type = 'search'; input.autocomplete = 'off';
  input.placeholder = zh ? '审批、幂等、TI-033…' : 'approval, invariant, TI-033…';
  const status = document.createElement('p'); status.setAttribute('role', 'status');
  const results = document.createElement('ol');
  panel.append(label, input, status, results); main.prepend(panel);
  let records, sequence = 0;
  const index = fetch('ti-search-index.json').then(r => {
    if (!r.ok) throw new Error('Search index unavailable');
    return r.json();
  }).then(value => { records = value; return value; });
  async function search() {
    const current = ++sequence, query = input.value.trim();
    results.replaceChildren();
    if (!query) { status.textContent = zh ? '输入中文、英文或条款编号。' : 'Enter a word or clause ID.'; return; }
    status.textContent = zh ? '正在搜索…' : 'Searching…';
    try {
      await index;
      if (current !== sequence) return;
      const terms = query.toLowerCase().split(/\s+/);
      const matches = records.filter(r => terms.every(t => r.text.toLowerCase().includes(t)))
        .sort((a,b) => Number(b.title.toLowerCase().includes(terms[0])) - Number(a.title.toLowerCase().includes(terms[0])));
      status.textContent = zh ? `找到 ${matches.length} 个章节` : `${matches.length} chapters found`;
      for (const record of matches) {
        const li = document.createElement('li'), a = document.createElement('a'), snippet = document.createElement('p');
        a.textContent = record.title; a.href = record.url;
        const clause = query.match(/^TI-\d{3}$|^TC-\d{3}$/i);
        if (clause && record.anchors.includes(clause[0].toLowerCase())) a.hash = clause[0].toLowerCase();
        const start = Math.max(0, record.text.toLowerCase().indexOf(terms[0]) - 45);
        snippet.textContent = (start ? '…' : '') + record.text.slice(start, start + 180) + '…';
        li.append(a, snippet); results.append(li);
      }
    } catch {
      if (current === sequence) status.textContent = zh ? '搜索暂不可用，请使用章节目录。' : 'Search unavailable. Use the chapter list.';
    }
  }
  function toggle(open) {
    panel.hidden = !open; button.setAttribute('aria-expanded', String(open));
    if (open) { input.focus(); search(); panel.scrollIntoView({block:'start'}); }
  }
  button.addEventListener('click', () => toggle(panel.hidden));
  input.addEventListener('input', search);
  input.addEventListener('keydown', e => { if (e.key === 'Enter') { e.preventDefault(); search(); } });
  document.addEventListener('keydown', e => {
    if (e.key === 'Escape' && !panel.hidden) { toggle(false); button.focus(); }
    if (e.key === '/' && !e.ctrlKey && !e.metaKey && !e.altKey && !/^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName) && !document.activeElement.isContentEditable) {
      e.preventDefault(); toggle(true);
    }
  });
})();
