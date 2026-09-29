# Dynamic reply voices

Settings → General → **Как будет говорить ассистент** selects one of two modes.

- **Обученный голос XTTS** uses the approved one-epoch local checkpoint named
  in `trained-checkpoint.txt`, plus the four approved WAV references in
  `xtts-references/`. The checkpoint is kept under
  `tools/voice_training/xtts_pilot_output/`, not bundled or committed to Git.
  It plays through the Windows default output device, without Voicemod. The
  pilot used 26 short clips whose transcripts still need human verification.
- **Silero + Voicemod** sends Silero speech to VB-CABLE Input. Set Voicemod's
  input to CABLE Output, select the Evil AI preset there, and keep Voicemod's
  output on the headphones. Jarvis does not select a Voicemod preset itself.
  Silero now stays loaded in one local worker process for subsequent replies;
  the first load still takes several seconds.
  Voicemod's monitoring must remain on to hear its processed CABLE input;
  with CABLE as Voicemod input, that stream contains Jarvis rather than the
  physical microphone.

The **Слышать себя в наушниках** switch is a separate local sidetone of the
physical microphone to the default headphones. It is off by default and does
not depend on Voicemod's own monitoring button. Virtual inputs/outputs are
rejected to avoid feedback. This sidetone does not apply the Voicemod effect
to the user's voice.

This setting affects generated TTS replies, including chat and voice dialogue.
Pre-recorded command replies remain unchanged. The XTTS model is subject to
the [Coqui Public Model License](https://github.com/coqui-ai/TTS/blob/dev/docs/source/models/xtts.md);
check its terms before distributing or using the model outside a personal test.

The current integration starts the Python model for each XTTS reply, so it can
have a noticeable delay. If this becomes a daily-use mode, keep the model warm
in a dedicated local voice process instead of reloading it for every sentence.
