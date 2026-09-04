use std::process::ExitCode;

use nme_core::FieldKey;

use crate::cli::SetArgs;

pub fn run(args: SetArgs) -> ExitCode {
    let Some(key) = FieldKey::from_cli_name(&args.metadata_name) else {
        eprintln!(
            "nme: unknown metadata field {:?}; field names are lowercase",
            args.metadata_name
        );
        return ExitCode::FAILURE;
    };

    let mut had_error = false;
    for path in &args.files {
        let mut doc = match nme_core::format::open(path) {
            Ok(doc) => doc,
            Err(err) => {
                eprintln!("nme: {}: {err}", path.display());
                had_error = true;
                continue;
            }
        };

        if !doc.fields().contains(&key) {
            eprintln!(
                "nme: metadata field {:?} is not supported for {}",
                args.metadata_name,
                path.display()
            );
            had_error = true;
            continue;
        }

        doc.set(key, Some(args.metadata_value.clone()));
        if let Err(err) = doc.save() {
            eprintln!("nme: {}: {err}", path.display());
            had_error = true;
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
