use pdfunite_tree::run;

/// Merge together all the PDFs in the input directory and its subdirectories (max 5 levels) into a single document.
/// If the flag `with-outlines`` is activated, the output file will be provided with a ToC (Table of Contents)
/// reflecting the structure of tree of the directory and its descendants. The tool does NOT modify the input
/// directory and its content.
///
/// Assumptions on the pdf tree:
/// 1. The tree has not more than 5 levels (the root is considered level 0).
/// 2. All the files in the input directory and its subdirectories are PDFs and their names are UT8-encoded.
/// 3. The PDFs in the directory and its subdirectories have at most these features:
///     * Pages
///     * PageMode
///
// (todo: specify rather which features are supported, and add more to them, otherwise is kind of lame).
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Directory containing the pdfs
    input_directory: String,
    /// Output path (must not be among the descendants of the input-directory)
    #[arg(short = 'o')]
    output_path: Option<String>,
    /// Provide the output file with a ToC (Oulines/Bookmark)
    /// reflecting the tree structure of the input directory.
    #[arg(short, long, default_value_t = true)]
    with_outlines: bool,
}

fn main() {
    // following minigrep from the official Rust book
    if let Err(err) = run() {
        eprintln!("Application error: {}", err);
        std::process::exit(1);
    }
}

pub fn run() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let mut target_dir_path = cli.input_directory;
    let target_dir_path = if target_dir_path.ends_with('/') {
        target_dir_path.pop();
        Path::new(&target_dir_path)
    } else {
        Path::new(&target_dir_path)
    }
    .canonicalize()?;

    let output_path = cli.output_path.unwrap_or(format!(
        "{}{DEFAULT_OUTPUT_SUFFIX}",
        target_dir_path.display()
    ));
    let output_path = Path::new(&output_path);

    if output_path.starts_with(&target_dir_path) {
        return Err(anyhow!(
            "The output file cannot be a descendant of the input directory: \
            '{}' is a descendant of '{}'",
            output_path.display(),
            target_dir_path.display()
        ));
    }

    let mut main_doc = get_merged_tree_doc(target_dir_path, cli.with_outlines)?;

    main_doc.compress();

    if std::fs::exists(output_path)? {
        return Err(anyhow!(
            "A file '{}' is already present",
            output_path.display()
        ));
    } else {
        main_doc.save(output_path)?;
        println!("Output document saved as '{}'", output_path.display());
    }

    Ok(())
}