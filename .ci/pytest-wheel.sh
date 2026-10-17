#!/bin/sh
# The suite every containerised wheel leg runs against the wheel in dist/.
#
# It lives here, not inside `docker run ... sh -c '...'`: a shell-quoted workflow body ends at the
# first apostrophe in a comment (measured on v0.18.0's first tag: `container's` closed the quote and
# the runner's toolcache python3 ran `-m virtualenv`), and the body is worth having one copy of.
#
# `git config` is here because the mounted checkout belongs to the runner, not to this container's
# user. The virtualenv's binaries are called by path instead of activating it: Alpine's /bin/sh has
# no `source`.
set -e

git config --global --add safe.directory '*'

python3 -m virtualenv .venv
.venv/bin/pip install -r .ci/requirements-test.txt pyrs-yaml --find-links dist --force-reinstall
.venv/bin/pytest tests/ -v
