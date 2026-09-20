import argparse
import sys
import torch
import sounddevice as sd

parser = argparse.ArgumentParser()
parser.add_argument("--text", required=True)
args = parser.parse_args()

device_name = next((d["name"] for d in sd.query_devices() if "CABLE Input" in d["name"]), None)
if not device_name:
    raise RuntimeError("CABLE Input (VB-Audio Virtual Cable) not found")

torch.set_num_threads(4)
model, _ = torch.hub.load(
    repo_or_dir="snakers4/silero-models",
    model="silero_tts",
    language="ru",
    speaker="v5_5_ru",
    trust_repo=True,
)
model.to(torch.device("cuda" if torch.cuda.is_available() else "cpu"))
audio = model.apply_tts(text=args.text, speaker="eugene", sample_rate=48000)
sd.play(audio.detach().cpu().numpy(), samplerate=48000, device=device_name)
sd.wait()
