use anyhow::{Context, Result};
use clap::Parser;
use log::info;
use std::fs;
use std::path::PathBuf;

// Use the shared internal function from the library crate.
use bibextract::extract_survey_internal;

/// A CLI for extracting survey content and bibliography from arXiv papers.
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// A list of arXiv paper IDs (e.g., '2104.08653').
    #[arg(short, long, required = true)]
    paper_ids: Vec<String>,

    /// The base path for the output files (e.g., 'survey').
    /// This will create 'survey.tex' and 'survey.bib'.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable verbose logging to see detailed processing steps.
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Configure logging based on the verbosity flag.
    let log_level = if args.verbose { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Use the shared internal function to process papers.
    match extract_survey_internal(args.paper_ids) {
        Ok((survey_text, bibtex)) => {
            // Handle file output.
            if let Some(output_path) = args.output {
                // Create the .tex file.
                let tex_path = output_path.with_extension("tex");
                fs::write(&tex_path, survey_text).with_context(|| format!("Failed to write to {:?}", tex_path))?;
                info!("Survey content written to {:?}", tex_path);

                // Create the .bib file.
                let bib_path = output_path.with_extension("bib");
                fs::write(&bib_path, bibtex).with_context(|| format!("Failed to write to {:?}", bib_path))?;
                info!("Bibliography written to {:?}", bib_path);
            } else {
                // If no output path is provided, print to stdout, separating the files.
                println!("--- survey.tex ---");
                println!("{}", survey_text);
                println!("--- bibliography.bib ---");
                println!("{}", bibtex);
            }
        }
        Err(e) => {
            log::error!("{}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

