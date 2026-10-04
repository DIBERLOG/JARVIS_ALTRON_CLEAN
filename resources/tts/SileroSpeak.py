import argparse
import json
import sys
import torch
import sounddevice as sd
from AudioOutput import select_output

parser = argparse.ArgumentParser()
parser.add_argument("--text")
parser.add_argument("--server", action="store_true")
args = parser.parse_args()
if not args.server and not args.text:
    parser.error("--text is required unless --server is used")

torch.set_num_threads(4)
model, _ = torch.hub.load(
    repo_or_dir="snakers4/silero-models",
    model="silero_tts",
    language="ru",
    speaker="v5_5_ru",
    trust_repo=True,
)
model.to(torch.device("cuda" if torch.cuda.is_available() else "cpu"))
def speak(text: str) -> None:
    audio = model.apply_tts(text=text, speaker="eugene", sample_rate=48000)
    sd.play(audio.detach().cpu().numpy(), samplerate=48000, device=select_output(sd))
    sd.wait()


if args.server:
    print("READY", flush=True)
    for line in sys.stdin:
        try:
            text = json.loads(line)["text"]
            if text.strip():
                speak(text)
            print("OK", flush=True)
        except Exception as error:
            print(f"ERROR {error}", flush=True)
else:
    speak(args.text)
