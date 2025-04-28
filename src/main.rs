pub mod latex;

use anyhow::{Context, Result};
use clap::Parser;
use log::info;
use std::fs;
use std::path::PathBuf;
use latex::{Bibliography, process_arxiv_paper};

/// CLI app for retrieving related work or background sections from arXiv papers
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// arXiv paper IDs (e.g., 2104.08653)
    #[arg(short, long)]
    paper_ids: Vec<String>,
    /// Output file (prints to stdout if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Configure logging
    if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    if args.paper_ids.is_empty() {
        anyhow::bail!("No paper IDs provided. Use --paper-ids option to specify at least one arXiv ID.");
    }

    let mut all_papers = Vec::new();
    let mut consolidated_bibliography = Bibliography::new();

    // Process each paper
    for paper_id in &args.paper_ids {
        info!("Processing arXiv paper with ID: {}", paper_id);
        // Download and process the paper
        let mut paper = process_arxiv_paper(paper_id)?;
        // Verify bibliography
        info!("Verifying bibliography entries for paper {}", paper_id);
        let verified_count = paper.verify_bibliography()?;
        info!("Verified {}/{} entries for paper {} using parallel verification", 
                verified_count, 
                paper.bibliography.iter().count(),
                paper_id);
        
        info!("Found {} sections with bibliography entries", paper.sections.len());
        // Add paper to our collection
        all_papers.push(paper);
    }

    // Merge bibliographies from all papers
    for paper in &all_papers {
        for entry in paper.bibliography.iter() {
            consolidated_bibliography.insert(entry.clone());
        }
    }

    // Process and format all sections with the consolidated bibliography
    let mut output = String::new();
    for paper in &all_papers {
        for section in &paper.sections {
            // Add section header and content as raw LaTeX
            output.push_str(&format!("\\section{{{}}}\n\n", section.title));
            // Normalize citations in the content
            let (normalized_content, _) = consolidated_bibliography.normalize_citations(&section.content)?;
            // Add raw LaTeX content
            output.push_str(&normalized_content);
            output.push_str("\n\n");
        }
    }

    // Add bibliography section in LaTeX format
    output.push_str(&consolidated_bibliography.to_string());

    // Write output to file or stdout
    if let Some(output_file) = &args.output {
        fs::write(output_file, output)
            .with_context(|| format!("Failed to write output to {:?}", output_file))?;
        info!("Output written to {:?}", output_file);
    } else {
        println!("{}", output);
    }

    Ok(())
}
