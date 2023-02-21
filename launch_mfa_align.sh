#!/bin/bash

dir_name=$1
sub_dir_name=$2

mfa align ../podcasts-dataset/${dir_name}/${sub_dir_name} ../StyleSpeech/lexicon/librispeech-lexicon.txt  english ../StyleSpeech/dataset/LibriTTS/TextGrid/${dir_name}/${sub_dir_name}/ -j 16 -v --clean
