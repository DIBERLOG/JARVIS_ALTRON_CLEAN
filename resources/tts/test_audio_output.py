import unittest
from unittest.mock import patch
import AudioOutput


class FakeAudio:
    class default:
        device = (0, 0)

    @staticmethod
    def query_devices():
        return [
            {'name': 'CABLE Input', 'max_output_channels': 2},
            {'name': 'LG HDMI', 'max_output_channels': 2},
            {'name': 'JBL Quantum350', 'max_output_channels': 2},
            {'name': 'Microphone JBL', 'max_output_channels': 0},
        ]


class AudioOutputTests(unittest.TestCase):
    @patch('AudioOutput.output_mode', return_value='direct')
    def test_direct_skips_cable_and_prefers_headphones(self, _):
        self.assertEqual(AudioOutput.select_output(FakeAudio), 2)

    @patch('AudioOutput.output_mode', return_value='voicemod')
    def test_voicemod_uses_cable(self, _):
        self.assertEqual(AudioOutput.select_output(FakeAudio), 0)


if __name__ == '__main__':
    unittest.main()
