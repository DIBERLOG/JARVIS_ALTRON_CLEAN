"""Render one standalone preview without changing the Jarvis application."""

from pathlib import Path

from TTS.api import TTS


root = Path(__file__).resolve().parent
run = next((root / "smoke_output").glob("jarvis_ru_current_continue*"))
output = root / "jarvis_ru_current_preview_after_4_epochs.wav"
model = TTS(model_path=str(run / "best_model.pth"), config_path=str(run / "config.json"), gpu=True)
audio = model.synthesizer.tts(text="Здравствуйте, сэр. Я готов помочь.")
model.synthesizer.save_wav(audio, str(output))
print(output)
