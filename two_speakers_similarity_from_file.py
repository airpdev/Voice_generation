import argparse

from pathlib import Path
import numpy as np

parser = argparse.ArgumentParser(description='Compute similarity using csv saved voice code vectors')
parser.add_argument('-csv1', required=False, help='Path to the speaker1 csv voice code', type=str,
                    default = 'speaker1.csv')
parser.add_argument('-csv2', required=False, help='Path to the speaker2 csv voice code', type=str,
                    default = 'speaker2.csv')
parser.add_argument('-out', required=False, help='File name to save the similarity value', type=str,
                    default = 'similarity.csv')
args = parser.parse_args()

csv1 = np.genfromtxt(args.csv1, delimiter=',')

csv2 = np.genfromtxt(args.csv2, delimiter=',')

similarity = np.inner(csv1, csv2)

np.savetxt(args.out, [similarity])