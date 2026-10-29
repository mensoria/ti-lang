(() => {
  const match = location.pathname.match(/^(.*\/)(en|zh-CN)\/(.*)$/);
  if (!match) return;
  const language = match[2];
  const zh = language === 'zh-CN';
  try { localStorage.setItem('ti-language', language); } catch { /* Optional preference. */ }

  const title = document.querySelector('.menu-title');
  if (title) {
    const home = document.createElement('a');
    home.className = 'ti-brand'; home.href = 'index.html'; home.textContent = 'Ti Lang';
    const label = document.createElement('span');
    label.className = 'ti-brand-section'; label.textContent = zh ? '文档' : 'Docs';
    title.replaceChildren(home, label);
  }

  const toolbar = document.querySelector('.right-buttons');
  if (!toolbar) return;
  const control = document.createElement('details');
  control.className = 'ti-language';
  const summary = document.createElement('summary');
  summary.textContent = zh ? '简体中文' : 'English';
  summary.setAttribute('aria-label', zh ? '选择语言' : 'Select language');
  // A single control owns language selection throughout the reader.
  const choices = document.createElement('div');
  choices.className = 'ti-language-options';
  for (const [code, name] of [['zh-CN', '简体中文'], ['en', 'English']]) {
    const link = document.createElement('a');
    link.textContent = name; link.lang = code; link.hreflang = code;
    const update = () => {
      const anchor = /^#(?:ti|tc)-\d{3}$/.test(location.hash) ? location.hash : '';
      link.href = `${match[1]}${code}/${match[3]}${anchor}`;
    };
    update(); window.addEventListener('hashchange', update);
    if (code === language) link.setAttribute('aria-current', 'true');
    choices.append(link);
  }
  control.append(summary, choices); toolbar.prepend(control);
  document.addEventListener('click', event => {
    if (!control.contains(event.target)) control.open = false;
  });
  document.addEventListener('keydown', event => {
    if (event.key === 'Escape' && control.open) {
      control.open = false; summary.focus();
    }
  });
})();
