// The till's beeps (CHK-22): a short high beep for a good scan, a lower, longer one
// for a problem. Made with the browser's Web Audio, so there is no sound file to ship.
// Where the browser has no sound (tests), nothing happens.

let audio: AudioContext | null = null;

function play(frequency: number, milliseconds: number) {
  if (typeof AudioContext === "undefined") {
    return;
  }
  audio ??= new AudioContext();
  const tone = audio.createOscillator();
  const volume = audio.createGain();
  tone.frequency.value = frequency;
  volume.gain.value = 0.1;
  tone.connect(volume).connect(audio.destination);
  tone.start();
  tone.stop(audio.currentTime + milliseconds / 1000);
}

// A good scan: short and high.
export function beepGood() {
  play(1800, 80);
}

// A problem: lower and longer, so it can't be mistaken for a good scan.
export function beepProblem() {
  play(300, 350);
}
