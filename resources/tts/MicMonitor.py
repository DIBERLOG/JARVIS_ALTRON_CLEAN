"""Optional local microphone sidetone, independent of the Jarvis VB-CABLE route.

Only physical default devices are accepted to prevent virtual-cable feedback.
This is raw microphone monitoring, not a Voicemod effect.
"""

import argparse
import sounddevice as sd


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    default_input, output_index = sd.default.device
    if output_index < 0:
        raise RuntimeError("Default microphone or headphones are not configured")
    output_name = sd.query_devices(output_index)["name"]
    virtual_names = ("cable", "voicemod", "virtual")
    if any(part in output_name.lower() for part in virtual_names):
        raise RuntimeError(f"Refusing to play microphone into a virtual output: {output_name}")
    inputs = [(index, device["name"]) for index, device in enumerate(sd.query_devices())
              if device["max_input_channels"] > 0
              and not any(part in device["name"].lower() for part in virtual_names)
              and ("microphone" in device["name"].lower() or "микрофон" in device["name"].lower())]
    if not inputs:
        raise RuntimeError("No physical microphone is available for sidetone")
    input_index, input_name = next(((index, name) for index, name in inputs if index == default_input),
                                   inputs[0])

    def pass_through(indata, outdata, frames, time_info, status):
        if status:
            print(status, flush=True)
        outdata[:] = indata

    print(f"Monitoring {input_name} -> {output_name}", flush=True)
    if args.check:
        sd.check_input_settings(device=input_index, channels=1, samplerate=48000)
        sd.check_output_settings(device=output_index, channels=1, samplerate=48000)
        return
    with sd.Stream(device=(input_index, output_index), channels=1, samplerate=48000,
                   blocksize=512, latency="low", callback=pass_through):
        while True:
            sd.sleep(1000)


if __name__ == "__main__":
    main()
