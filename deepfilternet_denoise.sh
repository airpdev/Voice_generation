#!/bin/bash

export PATH=$PATH:/home/ubuntu/.local/bin
export WORKON_HOME=~/.virtualenvs
export PYTHONPATH=DeepFilterNet/DeepFilterNet

source ~/.local/bin/virtualenvwrapper.sh

workon deepfilternet

PARAM=$1

python DeepFilterNet/DeepFilterNet/df/enhance.py ${PARAM}

deactivate
