use serde::Deserialize;
use std::{collections::BTreeMap, fs, process::ExitCode};
use ti_lang::{Actor, Record, Value};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    entity: String,
    action: String,
    actor: Actor,
    input: BTreeMap<String, Value>,
    old: Option<Record>,
    #[serde(default)]
    new_id: String,
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        return Err("usage: ti check|compile MODEL.ti | ti eval MODEL.ti REQUEST.json".into());
    }
    if fs::metadata(&args[2])?.len() > 262144 {
        return Err("TI_LIMIT: source exceeds 256 KiB".into());
    }
    let source = fs::read_to_string(&args[2])?;
    let model = ti_lang::compile(&source).map_err(|e| serde_json::to_string(&e).unwrap())?;
    match args[1].as_str() {
        "check" => println!("validated {} ({})", args[2], model.source_sha256),
        "compile" => println!("{}", serde_json::to_string_pretty(&model)?),
        "eval" => {
            if fs::metadata(args.get(3).ok_or("missing request")?)?.len() > 2097152 {
                return Err("TI_LIMIT: harness request exceeds 2 MiB".into());
            }
            let r: Request =
                serde_json::from_str(&fs::read_to_string(args.get(3).ok_or("missing request")?)?)?;
            let result = model
                .evaluate(
                    &r.entity,
                    &r.action,
                    &r.actor,
                    &r.input,
                    r.old.as_ref(),
                    &r.new_id,
                )
                .map_err(|e| serde_json::to_string(&e).unwrap())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => return Err("unknown command".into()),
    }
    Ok(())
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
