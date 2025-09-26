pub const HELP: &str = "\
USAGE:
    line_endings [options] file_pattern...

OPTIONS:
    -h, --help                   Prints help information
    -f, --folder <FOLDER>        Specify the folder to search in (default: current directory)
    -c, --case-sensitive         Case-sensitive glob matching
    -b, --bom                    Check for Byte Order Mark (BOM) in files
    -r, --recursive              Recursively search subdirectories

FIXES:
    -w, --windows-line-endings   Rewrite with Windows line endings (CRLF)
    -l, --linux-line-endings     Rewrite with Linux line endings (LF)
    -m, --remove-bom             Remove BOM from files that have one";

/// Show help message
pub fn show_help() {
    println!("{HELP}");
}
