#!/bin/bash

# Point this to the root directory
export ROOT_SCRIPTS_PATH=/home/ubuntu/work/voice_generation/voice_generation

export PATH=$PATH:/home/ubuntu/.local/bin
export WORKON_HOME=~/.virtualenvs
source ~/.local/bin/virtualenvwrapper.sh

workon s3prl

OUT_YAML=$1

S3PRL=s3prl/s3prl
cd ${S3PRL}

./downstream/a2a-vc-vctk/custom_decode_bhuman.sh ar_taco2 vq_wav2vec 50000 downstream/a2a-vc-vctk/hifigan_vctk ${OUT_YAML}

deactivate