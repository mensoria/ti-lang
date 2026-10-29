# Book authoring and GitHub Pages publication

## Source ownership

The English [standard](ti-language-standard.md) is normative. Its complete
[Simplified Chinese translation](ti-language-standard.zh-CN.md) uses the same
TI-001–TI-054 identifiers and identical EBNF. The conformance matrices use the same
44 case IDs. Technical identifiers, syntax, examples, tags, and error codes are
not translated. Resolve translation discrepancies against English and update
both manuscripts together.

`docs/` and `examples/` are maintained sources. `scripts/build_book.py` generates
21 matching chapters per language into ignored `book/{en,zh-CN}/src/`; edit the
manuscripts, not generated chapters. Configuration, theme and landing page live
under `book/`; `book/{en,zh-CN}/home.md` owns the informative reader homepages. Generated site/build evidence are ignored outputs under `site/`
and `dist/`. The build produces online HTML reading editions only.

## Local build

Requires Python 3.11+ and official mdBook 0.5.4. No Node package installation is
required. Bootstrap supports macOS arm64/x64 and Linux arm64/x64, verifies a pinned
release archive SHA-256, and writes only an ignored local cache.

```sh
python3 scripts/bootstrap_mdbook.py
python3 scripts/build_book.py
python3 -m http.server 8000 --bind 127.0.0.1 --directory site
```

Open `http://127.0.0.1:8000/` for the landing page. The production project base path
is `/ti-lang/`; serve under that prefix when testing the production 404 route.
The build checks clause/case coverage, identical grammar, manuscript links,
generated HTML targets and anchors, assets, and search output. It does not
prove translation quality or Ti execution conformance.

## GitHub Pages

Target: [mensoria.github.io/ti-lang](https://mensoria.github.io/ti-lang/).
English is under `/en/`; Simplified Chinese under `/zh-CN/`. The root opens the
reader directly, defaulting to Simplified Chinese and remembering an explicitly
selected edition when local storage is available. The top-right language selector
is the only edition-selection control. It stays on the same chapter; stable TI/TC
anchors survive switching. There is no separate language-selection landing page.

In repository **Settings → Pages → Build and deployment → Source**, select
**GitHub Actions**. The `Documentation book` workflow builds/checks pull requests
without deployment; pushes to `main` and manual runs on `main` publish after a
successful build. Pages permissions are restricted to the deployment job.
Environment protection rules still apply. No custom domain is configured.

The workflow uploads both the deployable site and a build report. Its action
revisions are pinned. When updating mdBook, review the official asset checksums,
theme behavior, generated link checks, and workflow before replacing the pin.

A local successful build is not remote deployment evidence. Confirm the matching
commit's build/deploy jobs and inspect the public bilingual site before reporting
publication complete. A published draft does not establish parser, evaluator,
host, or production conformance.

Upstream operational references:
[mdBook CI](https://rust-lang.github.io/mdBook/continuous-integration.html) and
[GitHub Pages custom workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).
