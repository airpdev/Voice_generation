#!/bin/bash

export PATH=$PATH:/home/ubuntu/.local/bin
export WORKON_HOME=~/.virtualenvs
source ~/.local/bin/virtualenvwrapper.sh

workon thefuzz

transcript1=$1
transcript2=$2

THEFUZZ_PATH=../thefuzz
cd ${THEFUZZ_PATH}

python compare.py ${transcript1} ${transcript2}

deactivate
