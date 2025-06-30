# /// script
# dependencies = ["fastmcp", "bibextract"]
# ///

from fastmcp import FastMCP

mcp = FastMCP("Get arvix paper related work and bibliography")


@mcp.tool(
    name="arvix_survey", description="Get arvix paper related work and bibliography"
)
def arvix_survey(arvix_id: str) -> str:
    """
    Get arvix paper related work and bibliography.
    Args:
        arvix_id (str): The ID of the arXiv paper to process.
    Returns:
        str: related work and cited work bibliography
    """
    try:
        from bibextract import extract_survey

        result = extract_survey([arvix_id])
        return f"Related work:\n{result['survey_text']}\n\nBibliography:\n{result['bibtex']}"
    except Exception as e:
        return f"Error processing arXiv paper {arvix_id}: {e}"


if __name__ == "__main__":
    mcp.run()
