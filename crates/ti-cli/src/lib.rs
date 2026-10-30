use serde::Deserialize;
use std::{collections::BTreeMap, fs, process::ExitCode};
use ti_language::{Actor, Record, Value};
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
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().any(|a| a == "--manifest") {
        return project_run(args);
    }
    if args.len() < 3 {
        return Err(
            "usage: ti check|compile|freeze MODEL.ti | ti eval MODEL.ti REQUEST.json".into(),
        );
    }
    if fs::metadata(&args[2])?.len() > 262144 {
        return Err("TI_LIMIT: source exceeds 256 KiB".into());
    }
    let source = fs::read_to_string(&args[2])?;
    let model = ti_language::compile(&source).map_err(|e| serde_json::to_string(&e).unwrap())?;
    match args[1].as_str() {
        "check" => println!("validated {} ({})", args[2], model.source_sha256),
        "compile" => println!("{}", serde_json::to_string_pretty(&model)?),
        "freeze" => println!(
            "{}",
            serde_json::to_string_pretty(
                ti_artifact::Artifact::freeze(&source, Default::default())
                    .map_err(|e| serde_json::to_string(&e).unwrap())?
                    .manifest()
            )?
        ),
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
pub fn main_entry() -> ExitCode {
    let args: Vec<_> = std::env::args().collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn project_run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::PathBuf;
    let mut manifest = None;
    let mut roots = vec![];
    let mut request = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--manifest" => {
                if manifest.is_some() {
                    return Err("duplicate --manifest".into());
                }
                i += 1;
                manifest = Some(PathBuf::from(args.get(i).ok_or("missing manifest path")?));
            }
            "--workspace-root" => {
                i += 1;
                roots.push(PathBuf::from(args.get(i).ok_or("missing workspace root")?));
            }
            value if args[1] == "eval" && !value.starts_with('-') && request.is_none() => {
                request = Some(PathBuf::from(value))
            }
            _ => return Err("unknown project argument".into()),
        }
        i += 1;
    }
    let manifest = manifest.ok_or("--manifest required")?;
    let diagnostic = |e: ti_language::Diagnostic| serde_json::to_string(&e).unwrap();
    let project = if args[1] == "lock" {
        ti_project::lock(&manifest, &roots)
    } else {
        ti_project::open(&manifest, &roots)
    }
    .map_err(diagnostic)?;
    if args[1] == "lock" {
        println!(
            "locked {} ({})",
            project.lockfile().root_package.name,
            project.lockfile().graph_sha256
        );
        return Ok(());
    }
    let model = project.compile().map_err(diagnostic)?;
    match args[1].as_str() {
        "check" => println!(
            "validated project {} ({})",
            project.lockfile().root_package.name,
            project.lockfile().graph_sha256
        ),
        "compile" => println!("{}", serde_json::to_string_pretty(&model)?),
        "eval" => {
            let path = request.ok_or("missing evaluator harness request")?;
            if fs::metadata(&path)?.len() > 2097152 {
                return Err("TI_LIMIT: harness request exceeds 2 MiB".into());
            }
            let r: Request = serde_json::from_str(&fs::read_to_string(path)?)?;
            let candidate = model
                .evaluate(
                    &r.entity,
                    &r.action,
                    &r.actor,
                    &r.input,
                    r.old.as_ref(),
                    &r.new_id,
                )
                .map_err(diagnostic)?;
            println!("{}", serde_json::to_string_pretty(&candidate)?);
        }
        _ => {
            return Err(
                "TI_EFFECT: project artifacts are not supported by the legacy freeze/host protocol"
                    .into(),
            )
        }
    }
    Ok(())
}
