#!/bin/bash

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

cp $SCRIPT_DIR/pre-commit $SCRIPT_DIR/../.git/hooks/
