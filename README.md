# bibextract

A Python package (with Rust backend) for extracting survey content and bibliography from arXiv papers.

## Features

- **Download arXiv papers**: Automatically downloads and extracts LaTeX source files from arXiv
- **Extract relevant sections**: Identifies and extracts Related Work, Background, and other survey-relevant sections
- **Bibliography management**: Parses and normalizes bibliography entries from multiple papers
- **BibTeX generation**: Outputs proper BibTeX format for all cited works
- **Citation verification**: Verifies citations against DBLP and arXiv databases
- **Parallel processing**: Uses Rust's parallel processing for fast bibliography verification

## Installation

### From PyPI (when published)

```bash
pip install bibextract
```

### From Source

1. Install Rust (if not already installed):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

2. Install maturin:

```bash
pip install maturin
```

3. Clone and build:

```bash
git clone https://github.com/your-username/bibextract.git
cd bibextract
maturin develop
```

## Usage

### Python API

```python
import bibextract

# Process one or more arXiv papers
result = bibextract.extract_survey(['2104.08653', '1912.02292'])

# Access the extracted content
survey_text = result['survey_text']  # Raw LaTeX with sections
bibtex = result['bibtex']           # BibTeX bibliography

# Save to files
with open('survey.tex', 'w') as f:
    f.write(survey_text)

with open('bibliography.bib', 'w') as f:
    f.write(bibtex)
```

### Command Line (original Rust binary)

```bash
# Build the CLI tool
cargo build --release

# Process papers
./target/release/bibextract --paper-ids 2104.08653 1912.02292 --output survey.tex
```

## Example Output

The package will generate:

1. **Survey Text** (`survey_text`): LaTeX content with normalized sections:

```latex
\section{Related Work}

Recent advances in machine learning have shown...
\cite{author2021paper, smith2020method}

\section{Background}

The foundations of this work build upon...
\cite{jones2019foundation}
```

2. **BibTeX Bibliography** (`bibtex`): Properly formatted citations:

```bibtex
@article{author2021paper,
  title = {A Novel Approach to Machine Learning},
  author = {Author, First and Author, Second},
  journal = {Journal of ML},
  year = {2021},
}

@inproceedings{smith2020method,
  title = {Efficient Methods for Deep Learning},
  author = {Smith, John},
  booktitle = {Conference on AI},
  year = {2020},
}
```

## API Reference

### `extract_survey(arxiv_ids)`

Extract survey content from arXiv papers.

**Parameters:**

- `arxiv_ids` (list): List of arXiv paper IDs (e.g., `['2104.08653', '1912.02292']`)

**Returns:**

- `dict` with keys:
  - `'survey_text'`: Raw LaTeX text with extracted sections and normalized citations
  - `'bibtex'`: BibTeX bibliography entries for all cited works

**Raises:**

- `TypeError`: If `arxiv_ids` is not a list
- `ValueError`: If `arxiv_ids` is empty or contains no valid IDs
- `RuntimeError`: If there's an error processing the papers

## Development

### Running Tests

```bash
cargo test
```

### Building Documentation

```bash
cargo doc --open
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [PyO3](https://pyo3.rs/) for Python-Rust integration
- Uses [reqwest](https://github.com/seanmonstar/reqwest) for HTTP requests
- Bibliography verification via DBLP and arXiv APIs
- **Parallel Processing**: Uses parallel verification for improved performance
- **Multiple Paper Support**: Processes multiple papers and consolidates their bibliographies

## Installation

### Prerequisites

- Rust (latest stable version)
- Internet connection for downloading papers and verifying citations

### Building from Source

```bash
git clone https://github.com/yourusername/bibextract.git
cd bibextract
cargo build --release
```

The binary will be available at `target/release/bibextract`.

## Usage

### Basic Usage

Extract related work sections from a single arXiv paper:

```bash
bibextract --paper-ids 2104.08653
```

### Multiple Papers

Process multiple papers at once:

```bash
bibextract --paper-ids 2104.08653 2203.15556 2307.09288
```

### Save to File

Save the output to a LaTeX file:

```bash
bibextract --paper-ids 2104.08653 2203.15556 --output survey.tex
```

### Verbose Logging

Enable detailed logging to see the processing steps:

```bash
bibextract --paper-ids 2104.08653 --verbose
```

### Command Line Options

- `--paper-ids` or `-p`: List of arXiv paper IDs (e.g., 2104.08653)
- `--output` or `-o`: Output file path (prints to stdout if not specified)
- `--verbose` or `-v`: Enable verbose logging

## How It Works

### 1. Paper Download and Extraction

The tool downloads LaTeX source files from arXiv and extracts them:

- Supports both ZIP and TAR.GZ archives
- Finds the main LaTeX file automatically
- Processes `\input` and `\include` commands recursively

### 2. Section Detection

Identifies relevant sections based on common patterns:

- "Related Work"
- "Background"
- "Literature Review"
- "Prior Work"
- "State of the Art"
- And many more variants

### 3. Bibliography Processing

Parses `.bbl` files to extract bibliography entries:

- Handles standard BibTeX formats
- Extracts author, title, year, and other metadata
- Supports both `\bibitem` and `\citeauthoryear` formats

### 4. Citation Verification

Verifies bibliography entries using external APIs:

- **DBLP API**: For academic paper verification
- **arXiv API**: For arXiv preprint verification
- **Parallel Processing**: Verifies multiple entries simultaneously
- **Smart Matching**: Uses title, author, and year for accurate matching

### 5. Output Generation

Produces clean LaTeX output:

- Normalized citation keys
- Consolidated bibliography
- Ready-to-use LaTeX sections

## Output Format

The tool generates LaTeX output with:

```latex
\section{Related Work}
...section content with normalized citations...

\section{Background}
...section content with normalized citations...

% Consolidated bibliography
Bibliography {
  smith_machine_learning_2020: article {
    author: "John Smith",
    title: "Machine Learning Approaches",
    year: "2020",
    verified_source: "DBLP",
  }
  ...
}
```

## Example

```bash
# Extract related work from a few machine learning papers
bibextract --paper-ids 2104.08653 2203.15556 2307.09288 --output ml_survey.tex --verbose
```

This will:

1. Download and extract LaTeX sources for each paper
2. Parse bibliography files
3. Verify citations using DBLP and arXiv APIs
4. Extract related work sections
5. Normalize citation keys
6. Consolidate bibliographies
7. Save the result to `ml_survey.tex`

## Architecture

The project is organized into several modules:

- `latex::bibliography`: Bibliography parsing and management
- `latex::citation`: Citation extraction and normalization
- `latex::parser`: LaTeX file parsing and archive extraction
- `latex::verification`: Bibliography verification using external APIs

## Dependencies

- `anyhow`: Error handling
- `clap`: Command line argument parsing
- `reqwest`: HTTP client for API requests
- `regex`: Regular expression support
- `rayon`: Parallel processing
- `serde_json`: JSON parsing for API responses
- `tempfile`: Temporary file management
- `zip`: ZIP archive extraction
- `tar` + `flate2`: TAR.GZ archive extraction

## Logging

The tool uses structured logging to show progress:

```
INFO Processing arXiv paper with ID: 2104.08653
INFO Downloading source files from arXiv for paper: 2104.08653
INFO Extracting ZIP archive
INFO Verifying bibliography entries for paper 2104.08653
INFO Verified entry: smith2020 (progress: 1/25)
INFO Found 3 sections with bibliography entries
INFO Output written to "survey.tex"
```

## Error Handling

The tool handles various error conditions gracefully:

- Invalid arXiv IDs
- Missing or corrupted source files
- Network failures during verification
- Malformed LaTeX files
- Missing bibliography files

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
