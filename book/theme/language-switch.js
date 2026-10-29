(() => {
  const match = location.pathname.match(/^(.*\/)(en|zh-CN)\/(.*)$/);
  if (!match) return;
  const other = match[2] === 'en' ? 'zh-CN' : 'en';
  const link = document.createElement('a');
  link.className = 'ti-language-switch';
  const stableAnchor = /^#(?:ti|tc)-\d{3}$/.test(location.hash) ? location.hash : '';
  link.href = `${match[1]}${other}/${match[3]}${stableAnchor}`;
  link.lang = other;
  link.hreflang = other;
  link.textContent = other === 'en' ? 'English' : '简体中文';
  link.setAttribute('aria-label', other === 'en' ? 'Read this chapter in English' : '阅读本章中文版');
  const toolbar = document.querySelector('.right-buttons');
  if (toolbar) toolbar.prepend(link);
  window.addEventListener('hashchange', () => {
    link.hash = /^#(?:ti|tc)-\d{3}$/.test(location.hash) ? location.hash : '';
  });
})();
