#!/usr/bin/env python3
"""Install the optional source adapter; never touches database schemas or data."""
import argparse
import os
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('source', type=Path, help='Mensoria surrealdb-rust-ai-baas root')
args = parser.parse_args()
adapter = args.source.resolve() / 'crates/baas-adapters'
cargo = adapter / 'Cargo.toml'
module = adapter / 'src/data.rs'
bridge = adapter / 'src/ti_language.rs'
source = (root / 'integrations/mensoria/ti_language.rs').read_text()
relative = os.path.relpath(root, adapter)
block = f'[dependencies.ti-lang]\npath = "{relative}"\nfeatures = ["mensoria-host"]\noptional = true\n\n'
declaration = '\n#[cfg(feature = "ti-language")]\n#[path = "ti_language.rs"]\nmod ti_language;\n'
cargo_text, module_text = cargo.read_text(), module.read_text()
if bridge.exists() and bridge.read_text() != source:
    raise SystemExit('Existing adapter differs; review it before replacing.')
if '[dependencies.ti-lang]' not in cargo_text:
    if '[features]' not in cargo_text or 'qualification = []' not in cargo_text:
        raise SystemExit('Unsupported Cargo layout; no files changed.')
    cargo_text = cargo_text.replace('[features]', block + '[features]', 1)
    cargo_text = cargo_text.replace('qualification = []', 'qualification = []\nti-language = ["dep:ti-lang"]', 1)
else:
    if block.strip() not in cargo_text or 'ti-language = ["dep:ti-lang"]' not in cargo_text:
        raise SystemExit('Existing dependency differs; no files changed.')
if 'mod ti_language;' not in module_text:
    module_text += declaration
elif declaration.strip() not in module_text:
    raise SystemExit('Existing module declaration differs; no files changed.')
cargo.write_text(cargo_text)
module.write_text(module_text)
bridge.write_text(source)
print('Installed optional ti-language source adapter. Database state unchanged.')
