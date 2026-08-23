//! `nme info` / `nme i` — print a file's metadata to stdout.
//!
//! Text mode is one `Label: value` row per field the file's format
//! supports (unset fields print with an empty value, so the output also
//! doubles as a quick "what can this format even hold" reference). JSON
//! mode (`-j`) is the same information shaped for scripts: always a
//! top-level array, one element per input file, so callers never have to
//! special-case "did I pass one file or several".

use std::path::PathBuf;
use std::process::ExitCode;

use nme_core::FieldKey;
use serde::Serialize;

use crate::cli::InfoArgs;

#[derive(Serialize)]
struct FileMetadataJson {
    path: PathBuf,
    fields: Vec<FieldJson>,
}

#[derive(Serialize)]
struct FieldJson {
    key: &'static str,
    value: Option<String>,
}

pub fn run(args: InfoArgs) -> ExitCode {
    let multiple = args.files.len() > 1;
    let mut had_error = false;
    let mut json_out = Vec::with_capacity(args.files.len());

    for (i, path) in args.files.iter().enumerate() {
        let doc = match nme_core::format::open(path) {
            Ok(doc) => doc,
            Err(err) => {
                eprintln!("nme: {}: {err}", path.display());
                had_error = true;
                continue;
            }
        };

        let fields: Vec<(FieldKey, Option<String>)> =
            doc.fields().iter().map(|&key| (key, doc.get(key))).collect();

        if args.json {
            json_out.push(FileMetadataJson {
                path: path.clone(),
                fields: fields
                    .into_iter()
                    .map(|(key, value)| FieldJson {
                        key: key.cli_name(),
                        value,
                    })
                    .collect(),
            });
        } else {
            if multiple {
                if i > 0 {
                    println!();
                }
                println!("==> {} <==", path.display());
            }
            for (key, value) in fields {
                println!("{}: {}", key.label(), value.as_deref().unwrap_or(""));
            }
        }
    }

    if args.json {
        // Errors above already reduced `json_out` to just the files that
        // opened successfully; still emit valid JSON for those rather than
        // aborting the whole command over one bad file among several.
        match serde_json::to_string_pretty(&json_out) {
            Ok(text) => println!("{text}"),
            Err(err) => {
                eprintln!("nme: failed to serialize JSON output: {err}");
                had_error = true;
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
