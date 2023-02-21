#!/bin/bash

eval "$(conda shell.bash hook)"
conda activate lipsync

PARAM=$1

LIPSYNC_PATH=../LipSync/Cython
cd ${LIPSYNC_PATH}

python lipsync_generate.py ${PARAM} 

conda deactivate
