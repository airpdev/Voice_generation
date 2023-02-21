#!/bin/bash

export PATH=$PATH:/home/ubuntu/.local/bin
export WORKON_HOME=~/.virtualenvs
source ~/.local/bin/virtualenvwrapper.sh

workon speechcloning

PARAM=$1

HUGGINGFACE_PATH=../SpeechCloning
cd ${HUGGINGFACE_PATH}

python app_single.py ${PARAM}

deactivate
