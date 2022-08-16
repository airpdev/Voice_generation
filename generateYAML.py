import argparse
import yaml
import pprint
import os

ROOT_SCRIPTS_PATH=os.environ['ROOT_SCRIPTS_PATH']
NAMES=ROOT_SCRIPTS_PATH+'/'+'Names'

def parseArgsYAML():
    parser = argparse.ArgumentParser(description='Generate Generate YAML file')
    parser.add_argument('-src_audio', required=False, help='Path to the source audio, use absolute path', type=str,
                        default = '/home/ubuntu/s3prl/s3prl/downstream/a2a-vc-vctk/data/vcc2020/TEF1/E10052.wav')
    parser.add_argument('-ref_audio', required=False, help='Path to the reference audio, use absolute path',
                        type=str, default='/home/ubuntu/s3prl/s3prl/downstream/a2a-vc-vctk/data/vcc2020/TEF1/E10052.wav')
    parser.add_argument('-out_audio', required=False, help='Name of the file with the output audio',
                        type=str, default='speaker1_speaker2')
    parser.add_argument('-out_yaml', required=False, help='Name of the YAML ouput file',
                        type=str, default='toyaml.yml')
    args = parser.parse_args()
    return args

def readYAML():
    with open('template.yaml') as f:
        template = yaml.safe_load(f)
    return template

def writeYAML(data, args):
    with open(args.out_yaml, 'w') as f:
        yaml.dump(data, f)


if __name__ == '__main__':
    args = parseArgsYAML()
    template = readYAML()
    source_audio=ROOT_SCRIPTS_PATH+'/'+args.src_audio
    reference_audio=NAMES+'/'+args.ref_audio
    template["anusha_carolyn"]["src"] = source_audio
    template["anusha_carolyn"]["ref"] = [reference_audio]
    template[args.out_audio] = template.pop("anusha_carolyn")
    writeYAML(template, args)