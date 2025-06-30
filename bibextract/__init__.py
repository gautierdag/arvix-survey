"""
bibextract - A Python package for extracting survey content and bibliography from arXiv papers.
"""

from importlib.metadata import version

# Import the Rust-implemented function directly.
from .bibextract import extract_survey

__version__ = version("bibextract")

__all__ = ["extract_survey"]
