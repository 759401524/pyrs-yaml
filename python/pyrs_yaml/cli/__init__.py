"""Command-line interface for pyrs-yaml.

This subpackage is intentionally dependency-free so that ``import
pyrs_yaml.cli`` never breaks the base install. The actual application lives in
``pyrs_yaml.cli.app`` and requires the optional ``cli`` extra::

    pip install pyrs-yaml[cli]
"""

import sys


def main():
    """Console-script entry point with a friendly error when cyclopts is absent."""
    try:
        from pyrs_yaml.cli.app import app
    except ImportError as exc:
        if exc.name != "cyclopts":
            raise
        sys.stderr.write(
            "The pyrs-yaml CLI requires 'cyclopts' (Python >= 3.10).\n"
            "Install it with:\n"
            "    pip install pyrs-yaml[cli]\n"
        )
        sys.exit(1)
    app()


if __name__ == "__main__":
    main()
