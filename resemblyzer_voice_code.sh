#!/bin/bash

export PATH=$PATH:/home/ubuntu/.local/bin
export WORKON_HOME=~/.virtualenvs
source ~/.local/bin/virtualenvwrapper.sh

workon resemblyzer

PARAM=$1

python generate_voice_code.py ${PARAM}

deactivate
