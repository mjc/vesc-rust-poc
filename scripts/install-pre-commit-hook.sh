#!/usr/bin/env sh
set -eu

git config core.hooksPath .githooks
printf 'Installed repo hooks from .githooks\n'
