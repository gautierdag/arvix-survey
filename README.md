# bibextract

[![codecov](https://codecov.io/gh/gautierdag/bibextract/branch/main/graph/badge.svg?token=NWHDJ22L8I)](https://codecov.io/gh/gautierdag/bibextract) [![tests](https://github.com/gautierdag/bibextract/actions/workflows/test.yml/badge.svg)](https://github.com/gautierdag/bibextract/actions/workflows/bibextract.yml) [![image](https://img.shields.io/pypi/l/bibextract.svg)](https://pypi.python.org/pypi/bibextract) [![image](https://img.shields.io/pypi/pyversions/bibextract.svg)](https://pypi.python.org/pypi/bibextract) [![PyPI version](https://badge.fury.io/py/bibextract.svg)](https://badge.fury.io/py/bibextract)

A Python package (with Rust backend) for extracting survey content and bibliography from arXiv papers.

There are a lot of ArXiv MCP tools already. This is another.

What it does differently is that it extracts content directly from the LaTeX source of the paper, rather than parsing the PDF.

It also focuses entirely on survey/background/related work sections. Right now this tool will ignore all the other sections.

Once it extracts the content, it also extracts looks at the BBL file and tries to reconstruct the .bibtex file and normalise the entries. Not all BBL files work (see the [tests/bbls](tests/bbls/) for examples). Once it has a title/author/year, it will try to look up the arXiv ID or DOI of the paper, and use that in the bibtex entry instead of the raw entry from the BBL file.

This citation normalisation means that you can pass multiple papers to it and it will extract the related work content and bibliography from all of them, merging them into a single output, with limited overlap.

The goal of this tool is to make it easy to get LLM agents to read/cite/write background sections of papers. In a loop, an agent could read a paper, extract the related work section, and then use all the ArXiv IDs in that section to extract the related work sections of those papers, and so on. This way, you can build a large corpus of related work content without having to manually search for papers.

## Some future todos

- [ ] push to Smithery
- [ ] improve test coverage
- [ ] add more `.bbl` files to tests
- [ ] improve the MCP docs for the tool
- [ ] add a CLI binding to run directly with uvx

## Installation

### Installing via Smithery

To install bibextract for Claude Desktop automatically via [Smithery](https://smithery.ai/server/@gautierdag/bibextract):

```bash
npx -y @smithery/cli install @gautierdag/bibextract --client claude
```

### fastMCP server implementation

```bash
uv run bibextract_mcp.py
```

### fastMCP from URL

```bash
# obviously check the file before running it, don't trust random scripts from the internet
uv run --python 3.12 https://raw.githubusercontent.com/gautierdag/bibextract/refs/heads/main/bibextract_mcp.py
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
