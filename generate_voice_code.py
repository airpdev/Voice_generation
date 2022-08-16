import argparse

from resemblyzer import preprocess_wav, VoiceEncoder
from itertools import groupby
from pathlib import Path
import numpy as np

parser = argparse.ArgumentParser(description='Generate Voice Code')
parser.add_argument('-spk1', required=False, help='Path to the speaker audio', type=str,
                    default = '/home/ubuntu/s3prl/s3prl/downstream/a2a-vc-vctk/data/vcc2020/TEF1/E10052.wav')
parser.add_argument('-output', required=False, help='Path to the output file to save the voice code',
                    type=str, default='./output.csv')
args = parser.parse_args()

fpath = Path(args.spk1)
wav1 = preprocess_wav(fpath)

encoder = VoiceEncoder()

# The embeddings are numpy.ndarrays
embed1 = encoder.embed_utterance(wav1)

print("Voice code:")
print(embed1)

np.savetxt(args.output, embed1, delimiter=",")