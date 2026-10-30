#!/usr/bin/env python3
"""Generate two chapter-aligned mdBook editions from authoritative manuscripts."""
from pathlib import Path
import hashlib
import json
import re
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
LANGUAGES = ('en', 'zh-CN')
STANDARD = 'ti-language-standard'
SLUGS = ['01-purpose', '02-lexical', '03-modules', '04-types', '05-expressions',
         '06-entities', '07-actions', '08-events', '09-operations', '10-evolution',
         '11-diagnostics', '12-extensions', 'a-grammar', 'b-evidence', 'c-fingerprint']


def read(name, lang):
    suffix = '' if lang == 'en' else '.zh-CN'
    return (ROOT / 'docs' / f'{name}{suffix}.md').read_text()


def chapters(lang):
    standard = read(STANDARD, lang)
    sections = re.split(r'(?=^## (?:\d+\.|Appendix [ABC]\.))', standard, flags=re.M)
    assert len(sections) == 16, 'Standard chapter count changed; review slug mapping'
    pages = {'index.md': (ROOT / 'book' / lang / 'home.md').read_text()}
    for slug, content in zip(SLUGS, sections[1:]):
        pages[slug + '.md'] = re.sub(r'^## ', '# ', content, count=1)
    for slug, source in [('overview', 'language'), ('conformance', 'conformance'),
                         ('integration', 'mensoria-integration'), ('roadmap', 'roadmap'),
                         ('project-and-packages','project-and-packages'),
                         ('compiler-architecture','compiler-architecture'),
                         ('implementation-plan','implementation-plan'),
                         ('package-conformance','package-conformance')]:
        pages[slug + '.md'] = read(source, lang)
    example = (ROOT / 'examples/approval.ti').read_text()
    if lang == 'zh-CN':
        example = example.replace('// Reference approval model for the evaluator and optional Mensoria host.',
                                  '// 求值器与可选 Mensoria 宿主的参考审批模型。')
    pages['approval.md'] = ('# Approval walkthrough\n\n' if lang == 'en' else '# 审批示例详解\n\n') + (
        'Reference model exercised by the Rust evaluator and native Mensoria host tests.\n\n'
        if lang == 'en' else 'Rust 求值器与 Mensoria 原生宿主测试使用的单级审批参考模型。\n\n') + '```text\n' + example + '```\n\n' + (
        'Draft → Pending → Approved / Rejected / Withdrawn. Draft editing stays in Draft.\n\n'
        'A reviewer cannot approve their own request. Concurrent approve/withdraw commands compete on one revision. '
        'Assignments see the old record; events see the candidate. Lost responses require original-operation reconciliation.\n'
        if lang == 'en' else 'Draft → Pending → Approved／Rejected／Withdrawn；草稿编辑保持 Draft。\n\n'
        '审批人不能批准自己的申请。并发批准与撤回竞争同一记录版本。赋值读旧记录，事件读候选记录。响应丢失后核对原操作。\n')
    # Rewrite manuscript links to generated book chapters, preserving one authority.
    for filename, content in list(pages.items()):
        content = re.sub(r'\*\*((?:TI|PKG|ARC|MIG)-\d{3})\.\*\*', lambda m: f'<a id="{m[1].lower()}"></a>\n\n{m[0]}', content)
        content = re.sub(r'^(\| )((?:TC|PC)-\d{3})( \|)', lambda m: f'{m[1]}<a id="{m[2].lower()}"></a>{m[2]}{m[3]}', content, flags=re.M)
        for stem, dest in [(STANDARD, 'index'), ('language', 'overview'), ('conformance', 'conformance'),
                           ('mensoria-integration', 'integration'), ('roadmap', 'roadmap'),
                           ('project-and-packages','project-and-packages'),
                           ('compiler-architecture','compiler-architecture'),
                           ('implementation-plan','implementation-plan'),
                           ('package-conformance','package-conformance')]:
            content = content.replace(f']({stem}.zh-CN.md)', f']({dest}.md)')
            content = content.replace(f']({stem}.md)', f']({dest}.md)')
        content = content.replace('](../examples/package-design/README.md)', '](https://github.com/mensoria/ti-lang/tree/main/examples/package-design)')
        content = content.replace('](../examples/approval.ti)', '](approval.md)')
        content = content.replace('](implementation.zh-CN.md)', '](https://github.com/mensoria/ti-lang/blob/main/docs/implementation.zh-CN.md)')
        content = content.replace('](qualification/projects-latest.json)', '](https://github.com/mensoria/ti-lang/blob/main/docs/qualification/projects-latest.json)')
        content = content.replace('](qualification/latest.json)', '](https://github.com/mensoria/ti-lang/blob/main/docs/qualification/latest.json)')
        content = content.replace('](implementation.md)', '](https://github.com/mensoria/ti-lang/blob/main/docs/implementation.md)')
        # Chinese source's English authority link must go to the English book.
        if lang == 'zh-CN':
            content = content.replace('[英文源文](index.md)', '[英文源文](../en/01-purpose.html)')
        footer = '\n\n---\n\n' + ('Draft 0.1 · No runtime conformance claimed.' if lang == 'en' else '草案 0.1 · 不声称运行时符合性。')
        pages[filename] = content.rstrip() + ('' if filename == 'index.md' else footer) + '\n'
    return pages


def generate(lang):
    pages = chapters(lang)
    source = ROOT / 'book' / lang / 'src'
    source.mkdir(parents=True, exist_ok=True)
    # Delete only generator-owned chapter files from previous builds.
    manifest_path = source / '.generated.json'
    if manifest_path.exists():
        for name in json.loads(manifest_path.read_text()):
            if name not in pages and Path(name).name == name:
                (source / name).unlink(missing_ok=True)
    for name, content in pages.items():
        (source / name).write_text(content)
    groups = [
        (None, ['index.md']),
        ('Getting started' if lang == 'en' else '入门', ['overview.md', 'approval.md']),
        ('Language standard' if lang == 'en' else '语言规范', [x + '.md' for x in SLUGS[:12]]),
        ('Projects and compiler design (proposal)' if lang == 'en' else '项目与编译器设计（提案）',
         ['project-and-packages.md','compiler-architecture.md','implementation-plan.md','package-conformance.md']),
        ('Reference and integration' if lang == 'en' else '参考与集成',
         [x + '.md' for x in SLUGS[12:]] + ['conformance.md', 'integration.md', 'roadmap.md']),
    ]
    lines = ['# Summary' if lang == 'en' else '# 目录', '']
    for group, filenames in groups:
        if group:
            lines.extend(['', '# ' + group, ''])
        for filename in filenames:
            title = pages[filename].splitlines()[0].removeprefix('# ')
            if filename == 'index.md':
                title = 'Documentation' if lang == 'en' else '文档首页'
            lines.append(f'- [{title}]({filename})')
    (source / 'SUMMARY.md').write_text('\n'.join(lines) + '\n')
    manifest_path.write_text(json.dumps(list(pages), indent=2) + '\n')
    return len(pages)


def main():
    counts = {lang: generate(lang) for lang in LANGUAGES}
    subprocess.run(['python3', str(ROOT / 'scripts/check_book.py')], check=True)
    executable = ROOT / '.cache/tools/mdbook'
    if not executable.is_file():
        found = shutil.which('mdbook')
        if not found:
            raise SystemExit('Run python3 scripts/bootstrap_mdbook.py first.')
        executable = Path(found)
    version = subprocess.check_output([str(executable), '--version'], text=True).strip()
    if version != 'mdbook v0.5.4':
        raise SystemExit(f'Expected mdbook v0.5.4; got {version}')
    for lang in LANGUAGES:
        destination = ROOT / 'site' / lang
        if destination.exists():
            shutil.rmtree(destination)  # Only builder-owned generated edition output.
        subprocess.run([str(executable), 'build', str(ROOT / 'book' / lang),
                        '--dest-dir', str(ROOT / 'site' / lang)], check=True)
    for lang in LANGUAGES:
        records = []
        for name, content in chapters(lang).items():
            text = re.sub(r'<[^>]+>', '', content)
            text = re.sub(r'[\\`#*|]', ' ', text)
            text = re.sub(r'\s+', ' ', text).strip()
            records.append({'title': content.splitlines()[0].removeprefix('# '),
                            'url': name.removesuffix('.md') + '.html', 'text': text,
                            'anchors': re.findall(r'<a id="([^"]+)"></a>', content)})
        (ROOT / 'site' / lang / 'ti-search-index.json').write_text(
            json.dumps(records, ensure_ascii=False) + '\n')
    # Fingerprint custom assets so returning readers receive the current UI.
    for page in (ROOT / 'site').rglob('*.html'):
        html = page.read_text()
        for asset in ('book.css', 'language-switch.js', 'search.js'):
            source = ROOT / 'book/theme' / asset
            digest = hashlib.sha256(source.read_bytes()).hexdigest()[:12]
            html = html.replace('../theme/' + asset + '"', '../theme/' + asset + '?v=' + digest + '"')
        page.write_text(html)
    shutil.copyfile(ROOT / 'book/landing.html', ROOT / 'site/index.html')
    (ROOT / 'site/.nojekyll').write_text('')
    (ROOT / 'site/404.html').write_text('<!doctype html><html lang="en"><meta charset="utf-8"><title>Page not found — Ti Lang</title><h1>Page not found / 页面不存在</h1><a href="/ti-lang/">Ti Lang documentation / 文档首页</a></html>\n')
    subprocess.run(['python3', str(ROOT / 'scripts/check_book.py'), '--html'], check=True)
    report = {'edition': 'ti/0.1', 'draft': '0.1', 'mdbook': '0.5.4', 'languages': counts,
              'normative_clauses': 54, 'conformance_cases': 44,
              'source_sha256': {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
                                for p in sorted((ROOT / 'docs').glob('*.md'))},
              'evidence': 'documentation_build_only', 'runtime_conformance': 'not_run',
              'ci': 'not_run_locally', 'deployment': 'not_confirmed_by_build'}
    (ROOT / 'dist').mkdir(exist_ok=True)
    (ROOT / 'dist/build-report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
    print('Built bilingual book: site/index.html')


if __name__ == '__main__':
    main()
