use anyhow::Result;
use pico_args::Arguments;

use crate::types::ConfigSettings;

/// Parses command line arguments and returns configuration settings.
///
/// # Errors
///
/// Returns an error if invalid arguments are provided or conflicting options are specified.
pub fn parse_args(mut args: Arguments) -> Result<ConfigSettings> {
    // Parse flags
    let case_sensitive = args.contains(["-c", "--case-sensitive"]);
    let set_linux = args.contains(["-l", "--linux-line-endings"]);
    let set_windows = args.contains(["-w", "--windows-line-endings"]);
    let check_bom = args.contains(["-b", "--bom"]);
    let remove_bom = args.contains(["-m", "--remove-bom"]);
    let recursive = args.contains(["-r", "--recursive"]);
    let delete_backups = args.contains(["-d", "--delete-backups"]);

    let folder: Option<String> = args.opt_value_from_str(["-f", "--folder"])?;

    if set_linux && set_windows {
        return Err(anyhow::anyhow!(
            "Cannot set both Linux and Windows line endings at the same time"
        ));
    }

    // Get all file paths from command line
    let mut file_paths = Vec::new();
    let mut unrecognized_switches = Vec::new();

    while let Ok(path) = args.free_from_str::<String>() {
        // Check if the argument starts with "-", which indicates it's likely a switch
        if path.starts_with('-') {
            unrecognized_switches.push(path);
        } else {
            file_paths.push(path);
        }
    }

    // check for switches collected by the free_from_str loop
    if !unrecognized_switches.is_empty() {
        return Err(anyhow::anyhow!(format!(
            "Unrecognized switches: {unrecognized_switches:?}"
        )));
    }

    // Check for any remaining unparsed arguments (extra switches)
    let extras = args.finish();
    if !extras.is_empty() {
        return Err(anyhow::anyhow!(format!(
            "Unrecognized switches: {extras:?}"
        )));
    }

    Ok(ConfigSettings {
        case_sensitive,
        set_linux,
        set_windows,
        check_bom: check_bom || remove_bom, // need to check BOM if removing it
        remove_bom,
        recursive,
        delete_backups,
        supplied_paths: file_paths,
        folder,
    })
}
