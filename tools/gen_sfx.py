#!/usr/bin/env python3
"""Procedural SFX generator for Castle Lanes (plan.md P0-3).

Synthesizes every game sound as a 22.05 kHz mono WAV using only the Python
standard library - no external assets, fully reproducible and license-clean.

Run from the repo root:

    python3 tools/gen_sfx.py
"""

import math
import os
import random
import struct
import wave

SAMPLE_RATE = 22_050
OUT_DIR = os.path.join(os.path.dirname(__file__), "..", "assets", "audio")

random.seed(0xC0571A4E)  # deterministic output; the file is the asset anyway


def write_wav(name, samples):
    path = os.path.join(OUT_DIR, f"{name}.wav")
    with wave.open(path, "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(SAMPLE_RATE)
        frames = b"".join(
            struct.pack("<h", max(-32767, min(32767, int(sample * 32767))))
            for sample in samples
        )
        wav.writeframes(frames)
    print(f"wrote {path} ({len(samples) / SAMPLE_RATE:.2f}s)")


def silence(duration):
    return [0.0] * int(duration * SAMPLE_RATE)


def env_ad(i, count, attack, release_pow=2.5):
    """Attack-decay envelope: fast rise, exponential-ish fall."""
    t = i / count
    attack_samples = max(1, int(attack * count))
    if i < attack_samples:
        return i / attack_samples
    return (1.0 - (i - attack_samples) / (count - attack_samples)) ** release_pow


def tone(duration, freq_fn, gain_fn, start_phase=0.0):
    samples = []
    phase = start_phase
    count = int(duration * SAMPLE_RATE)
    for i in range(count):
        freq = freq_fn(i / count)
        phase += 2.0 * math.pi * freq / SAMPLE_RATE
        samples.append(math.sin(phase) * gain_fn(i, count))
    return samples, phase


def noise_burst(duration, gain_fn, lowpass=0.25):
    samples = []
    value = 0.0
    count = int(duration * SAMPLE_RATE)
    for i in range(count):
        value += (random.uniform(-1.0, 1.0) - value) * lowpass
        samples.append(value * gain_fn(i, count))
    return samples


def mix(*layers):
    length = max(len(layer) for layer in layers)
    out = [0.0] * length
    for layer in layers:
        for i, sample in enumerate(layer):
            out[i] += sample
    peak = max(1e-6, max(abs(sample) for sample in out))
    if peak > 0.92:
        out = [sample * 0.92 / peak for sample in out]
    return out


def concat(*parts):
    out = []
    for part in parts:
        out.extend(part)
    return out


def delayed(part, duration):
    return silence(duration) + part


def sfx_ui_click():
    pop, _ = tone(0.05, lambda t: 1500 - 600 * t, lambda i, n: env_ad(i, n, 0.05) * 0.7)
    tick = noise_burst(0.02, lambda i, n: (1 - i / n) * 0.35, lowpass=0.6)
    return mix(pop, tick)


def sfx_build_place():
    thud, _ = tone(0.16, lambda t: 170 - 60 * t, lambda i, n: env_ad(i, n, 0.04) * 0.95)
    click = noise_burst(0.03, lambda i, n: (1 - i / n) * 0.4, lowpass=0.55)
    return mix(thud, click)


def sfx_build_error():
    buzz, _ = tone(
        0.16,
        lambda t: 210 if t < 0.5 else 160,
        lambda i, n: env_ad(i, n, 0.05) * 0.55,
    )
    return mix(buzz)


def sfx_unit_spawn():
    pop, _ = tone(
        0.09,
        lambda t: 420 + 380 * t,
        lambda i, n: env_ad(i, n, 0.25) * 0.5,
    )
    return mix(pop)


def sfx_melee_hit():
    hit = noise_burst(0.07, lambda i, n: env_ad(i, n, 0.06) * 0.9, lowpass=0.3)
    thump, _ = tone(0.07, lambda t: 130 - 50 * t, lambda i, n: env_ad(i, n, 0.05) * 0.8)
    return mix(hit, thump)


def sfx_ranged_shot():
    swoosh = noise_burst(
        0.11,
        lambda i, n: math.sin(math.pi * i / n) ** 1.6 * 0.7,
        lowpass=0.55,
    )
    ping, _ = tone(0.05, lambda t: 900 + 500 * t, lambda i, n: env_ad(i, n, 0.2) * 0.25)
    return mix(swoosh, ping)


def sfx_unit_death():
    fall, _ = tone(
        0.24,
        lambda t: 460 - 320 * t,
        lambda i, n: env_ad(i, n, 0.05, 1.6) * 0.6,
    )
    crumble = noise_burst(0.16, lambda i, n: env_ad(i, n, 0.05, 2.2) * 0.35, lowpass=0.18)
    return mix(fall, crumble)


def sfx_bounty_coin():
    ping_a, phase = tone(0.07, lambda t: 1320, lambda i, n: env_ad(i, n, 0.06) * 0.6)
    ping_b, _ = tone(0.1, lambda t: 1960, lambda i, n: env_ad(i, n, 0.06) * 0.5, phase)
    return concat(ping_a, delayed(ping_b, 0.035))


def sfx_castle_alarm():
    pulses = []
    for _ in range(3):
        horn, _ = tone(
            0.22,
            lambda t: 196 + 6 * math.sin(2 * math.pi * 7 * t),
            lambda i, n: env_ad(i, n, 0.18, 1.2) * 0.6,
        )
        pulses.append(horn)
    gap = silence(0.14)
    return concat(pulses[0], gap, pulses[1], gap, pulses[2])


def sfx_victory():
    notes = [(523.25, 0.16), (659.25, 0.16), (783.99, 0.16), (1046.5, 0.34)]
    out = []
    for index, (freq, duration) in enumerate(notes):
        note, _ = tone(
            duration,
            lambda t, f=freq: f,
            lambda i, n: env_ad(i, n, 0.08, 1.8) * 0.5,
        )
        out.append(delayed(note, 0.0 if index == 0 else 0.0))
    return concat(*out)


def sfx_defeat():
    notes = [(440.0, 0.2), (349.23, 0.2), (293.66, 0.42)]
    out = []
    for freq, duration in notes:
        note, _ = tone(
            duration,
            lambda t, f=freq: f,
            lambda i, n: env_ad(i, n, 0.08, 1.8) * 0.5,
        )
        out.append(note)
    return concat(*out)


def sfx_ambient_loop():
    """Slow 8s drone loop, intentionally quiet - a bed, not music."""
    duration = 8.0
    count = int(duration * SAMPLE_RATE)
    out = []
    drones = [82.41, 110.0, 164.81, 220.0]
    for i in range(count):
        t = i / SAMPLE_RATE
        sample = 0.0
        for index, freq in enumerate(drones):
            lfo = 0.5 + 0.5 * math.sin(2 * math.pi * (0.05 + 0.03 * index) * t + index)
            sample += math.sin(2 * math.pi * freq * t) * 0.16 * lfo
        shimmer = noise_burst_seen(i, 0.02)
        out.append(sample + shimmer)
    # make it loop cleanly by crossfading the seam
    fade = int(0.35 * SAMPLE_RATE)
    for i in range(fade):
        blend = i / fade
        out[i] = out[i] * blend + out[-fade + i] * (1 - blend)
    return out[: count - fade]


_noise_state = 0.0


def noise_burst_seen(_i, lowpass):
    global _noise_state
    _noise_state += (random.uniform(-1.0, 1.0) - _noise_state) * lowpass
    return _noise_state * 0.05


GENERATORS = {
    "ui_click": sfx_ui_click,
    "build_place": sfx_build_place,
    "build_error": sfx_build_error,
    "unit_spawn": sfx_unit_spawn,
    "melee_hit": sfx_melee_hit,
    "ranged_shot": sfx_ranged_shot,
    "unit_death": sfx_unit_death,
    "bounty_coin": sfx_bounty_coin,
    "castle_alarm": sfx_castle_alarm,
    "victory": sfx_victory,
    "defeat": sfx_defeat,
    "ambient_loop": sfx_ambient_loop,
}


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, generator in GENERATORS.items():
        write_wav(name, generator())


if __name__ == "__main__":
    main()
