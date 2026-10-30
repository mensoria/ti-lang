#!/usr/bin/env python3
"""Check bilingual coverage and local links; never claim Ti runtime validation."""
from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import unquote, urlsplit
import argparse
import json
import re

ROOT = Path(__file__).resolve().parents[1]

class Links(HTMLParser):
    def __init__(self):
        super().__init__(); self.ids=set(); self.links=[]
    def handle_starttag(self, tag, attrs):
        attrs=dict(attrs)
        if 'id' in attrs: self.ids.add(attrs['id'])
        for name in ('href','src'):
            if name in attrs: self.links.append(attrs[name])


def check_html():
    site=ROOT/'site'
    pages={}
    for path in site.rglob('*.html'):
        parsed=Links(); parsed.feed(path.read_text()); pages[path.resolve()]=parsed
    checked=0
    for path,parsed in pages.items():
        for link in parsed.links:
            url=urlsplit(link)
            if url.scheme or url.netloc or link.startswith('data:'): continue
            raw=unquote(url.path)
            if raw.startswith('/ti-lang/'): target=site/raw[len('/ti-lang/'):]
            elif raw.startswith('/'): continue
            else: target=path.parent/raw if raw else path
            if target.is_dir(): target=target/'index.html'
            target=target.resolve()
            assert target.is_relative_to(site.resolve()),(path,link,'outside output')
            assert target.exists(),(path,link,'missing target')
            if url.fragment and target in pages:
                assert unquote(url.fragment) in pages[target].ids,(path,link,'missing anchor')
            checked+=1
    for lang in ('en','zh-CN'):
        for name in ('index.html',):
            assert (site/lang/name).exists(),(lang,name)
        index = json.loads((site/lang/'ti-search-index.json').read_text())
        assert len(index) == 25
        for record in index:
            target = (site/lang/record['url']).resolve()
            assert target in pages
            assert all(anchor in pages[target].ids for anchor in record['anchors'])
        assert not (site/lang/'print.html').exists(), (lang, 'print output is disabled')
        edition_ids = set().union(*(page.ids for path,page in pages.items() if path.parent == (site/lang).resolve()))
        assert all(f'ti-{i:03}' in edition_ids for i in range(1,55))
        assert any('审批' in record['text'] for record in index) if lang == 'zh-CN' else any('approval' in record['text'].lower() for record in index)
    assert not list(site.rglob('*.epub')), 'EPUB is outside requested scope'
    print(f'HTML: {len(pages)} pages; {checked} local links/assets/anchors checked.')


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--html',action='store_true');args=parser.parse_args()
    en=(ROOT/'docs/ti-language-standard.md').read_text()
    zh=(ROOT/'docs/ti-language-standard.zh-CN.md').read_text()
    pattern=r'\*\*(TI-\d{3})\.\*\*'
    expected=[f'TI-{i:03}' for i in range(1,55)]
    assert re.findall(pattern,en)==expected
    assert re.findall(pattern,zh)==expected
    assert re.search(r'```ebnf\n.*?```',en,re.S).group()==re.search(r'```ebnf\n.*?```',zh,re.S).group()
    # Every normative section retains identifiers and shared protocol tokens.
    def clauses(text):
        starts=list(re.finditer(pattern,text))
        return {m[1]:text[m.start():starts[i+1].start() if i+1<len(starts) else text.index('## Appendix A.')] for i,m in enumerate(starts)}
    e,z=clauses(en),clauses(zh)
    for key in expected:
        tokens=set(re.findall(r'`([^`\n]+)`',e[key]))
        # Protocol tokens and limits are checked; natural-language translations differ.
        for token in tokens:
            if re.fullmatch(r'(?:TI_[A-Z_]+|[0-9]+(?: KiB)?|ti/0\.1|approval-core/0\.1)',token):
                assert token in z[key],(key,token)
    for stem,prefix,count in [('project-and-packages','PKG',26),('compiler-architecture','ARC',14),('implementation-plan','MIG',10)]:
        a=(ROOT/f'docs/{stem}.md').read_text(); b=(ROOT/f'docs/{stem}.zh-CN.md').read_text()
        ids=[f'{prefix}-{i:03}' for i in range(1,count+1)]
        assert re.findall(r'\*\*('+prefix+r'-\d{3})\.\*\*',a)==ids
        assert re.findall(r'\*\*('+prefix+r'-\d{3})\.\*\*',b)==ids
        assert re.findall(r'```[^\n]*\n(.*?)```',a,re.S)==re.findall(r'```[^\n]*\n(.*?)```',b,re.S),(stem,'translated code changed')
    for suffix in ('','.zh-CN'):
        proposal=(ROOT/f'docs/package-conformance{suffix}.md').read_text()
        assert re.findall(r'^\| (PC-\d{3}) \|',proposal,re.M)==[f'PC-{i:03}' for i in range(1,31)]
    for lang in ('en','zh-CN'):
        suffix='' if lang=='en' else '.zh-CN'
        cases=(ROOT/f'docs/conformance{suffix}.md').read_text()
        assert re.findall(r'^\| (TC-\d{3}) \|',cases,re.M)==[f'TC-{i:03}' for i in range(1,45)]
        assert all(x in expected for x in re.findall(r'TI-\d{3}',cases))
        src=ROOT/'book'/lang/'src'
        files=re.findall(r'\]\(([^)]+\.md)\)',(src/'SUMMARY.md').read_text())
        assert len(files)==25 and len(set(files))==25
        for filename in files:
            content=(src/filename).read_text()
            assert len(re.findall(r'^```',content,re.M))%2==0,(lang,filename)
            for link in re.findall(r'\]\(([^)]+)\)',content):
                u=urlsplit(link)
                if not u.scheme and u.path.endswith('.md'): assert (src/unquote(u.path)).exists(),(lang,filename,link)
    for path in [ROOT/'README.md',ROOT/'README.zh-CN.md',*(ROOT/'docs').glob('*.md'),*(ROOT/'examples').glob('*.ti')]:
        text=path.read_text();assert text.endswith('\n'),path
        assert all(line==line.rstrip() for line in text.splitlines()),path
        for link in re.findall(r'\]\(([^)]+)\)',text):
            u=urlsplit(link)
            if not u.scheme and u.path: assert (path.parent/unquote(u.path)).exists(),(path,link)
    print('Manuscripts: 54 matched clauses, identical EBNF, 44 matched case IDs, 25 chapters per language.')
    if args.html: check_html()


if __name__=='__main__': main()
