# bibextract

[![codecov](https://codecov.io/gh/gautierdag/bibextract/branch/main/graph/badge.svg?token=NWHDJ22L8I)](https://codecov.io/gh/gautierdag/bibextract) [![tests](https://github.com/gautierdag/bibextract/actions/workflows/test.yml/badge.svg)](https://github.com/gautierdag/bibextract/actions/workflows/bibextract.yml) [![image](https://img.shields.io/pypi/l/bibextract.svg)](https://pypi.python.org/pypi/bibextract) [![image](https://img.shields.io/pypi/pyversions/bibextract.svg)](https://pypi.python.org/pypi/bibextract) [![PyPI version](https://badge.fury.io/py/bibextract.svg)](https://badge.fury.io/py/bibextract)

A Python package (with Rust backend) for extracting survey content and bibliography from arXiv papers.

## Features

- **Download arXiv papers**: Automatically downloads and extracts LaTeX source files from arXiv
- **Extract relevant sections**: Identifies and extracts Related Work, Background, and other survey-relevant sections
- **Bibliography management**: Parses and normalizes bibliography entries from multiple papers
- **BibTeX generation**: Outputs proper BibTeX format for all cited works
- **Citation verification**: Verifies citations against DBLP and arXiv databases
- **Parallel processing**: Uses Rust's parallel processing for fast bibliography verification

## Installation

### MCP server implementation

```bash
uv run bibextract_mcp.py
```

### From PyPI

```bash
uv add bibextract
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
    git clone https://github.com/gautier/bibextract.git
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

## Development

### Running Tests

```bash
cargo test
pytest tests
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
