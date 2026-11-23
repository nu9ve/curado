# Mixer views and hardware-inspired layouts

Research and initial console view, 2026-10-29. The console is a visual approximation, not hardware equivalence. Product references describe their own equipment; curado's capabilities are established by its code and [Architecture](ARCHITECTURE.md).

**Update, ADR-024:** **cabina** now follows the hardware diagram's plan. Sections below that describe an earlier arrangement are history.

**Update, ADR-022 and ADR-023:** the console now has per-channel Sound Color FX, Beat FX on CH 1, CH 2 and master, trim, beat loop, beat jump and roll pads, slip, reverse, platter scratch, vinyl brake and eight samples. Where the sections below list these as gaps, this update replaces them. Input sources, booth, microphone and four channels remain gaps. These effects are curado's own and were not compared with hardware.

## Switch views during playback

Mix offers **RX3** (the default), **DDJ-SB**, **curado**, **Compact** and
**Monitor**. Compact fits the classic controls into less space. Monitor
emphasizes waveforms, elapsed time, BPM, key, meters, deck state, transport
and crossfader. The toolbar suggests DDJ-SB for a connected, recognized
DDJ-SB and keeps the user's selection. All views accept keyboard, pointer
and the current MIDI mappings. The DDJ-SB can drive RX3, and the DDJ-SB view
works with a different controller or with keyboard and pointer alone.

The selected view is stored locally. The existing native session continues
to own the loaded decks, playback positions, loops, sync, samples, and mixer
state. The same keyboard and MIDI actions remain available. Leaving a view
during an on-screen jog drag releases that gesture.

A separate desktop monitoring window remains a follow-up. Define its
readouts, focus and input ownership before adding another window. The
audio-reactive Metal visual is a separate feature.

## DDJ-SB view

The two-channel surface places Browse and LOAD A/B at the top, with FX
controls above the jogs, vertical tempo faders beside the platters, and four
mode-dependent pads over Play, CUE, SYNC and SHIFT. The central mixer keeps
three-band EQ, centered LP/HP Filter, channel cue, faders, meters and crossfader
together. Browse follows the active library list and LOAD uses its current
selection, including registered tracks from a device library.

The waveforms and platter readouts use native positions, playback BPM and
the estimated sounding key. The clock sits on an opaque background and the
workspace's Camelot/note choice applies. The layout scrolls at small widths
to preserve readable controls. FILTER uses the current Color FX state and
can be selected or bypassed explicitly on the channel. Channel cue, master cue and cue
mix use the same four-channel routing as RX3.

SHIFT in the view toggles the deck's alternate actions. It clears hot cues,
toggles the current auto loop, exposes the mapped manual-loop alternatives,
and cycles tempo range from KEY LOCK. SHIFT on a screen sampler pad replaces
it with the selected library track, using curado's existing sample workflow.
Physical mode buttons and SHIFT generate their own hardware messages. The
screen shows its own selected pad mode and reflects the resulting engine
state. See the [DDJ-SB mapping and feature inventory](MIDI.md#ddj-sb-v1-coverage)
and the [physical playback checklist](MIDI.md#ddj-sb-layout-and-playback-check).

## Current foundation

All Mix views use validated actions and existing snapshots. Keyboard and
MIDI retain their own bindings. Position comes from the engine, with 20 Hz
state snapshots and a 60 Hz position stream. Decks share eight persistent hot
cues per track and eight global one-shot sample slots. The UI exposes sync,
quantize, looping, scratch and bend, beat FX, Color FX, and independent cue
when the output has four channels. See [Architecture](ARCHITECTURE.md) and
[Status](STATUS.md) for contracts and verification.

## Original RX3 research and presentation

The following sections record the initial console research. Use the current foundation above and the MIDI inventory for present-day coverage.

The original Mix layout picker offered **curado / cabina**. The two-deck surface follows the broad organization of the [RX3 hardware diagram](https://downloads.support.alphatheta.com/software_info/all-in-one-dj-systems/XDJ-RX3/XDJ-RX3_HardwareDiagram_rekordbox_E1.pdf): a central waveform display, two vertical channel strips, crossfader, side jogs, transport and eight pads per deck, plus color and beat effect sections. It uses original CSS-drawn hardware, not a photograph or manufacturer assets. The chassis stays dark for the hardware look while accent and waveform colors follow the theme; the surrounding app still uses its chosen style.

Working controls use the existing actions and snapshots. Each deck independently selects HOT CUE or SAMPLER on the same eight pads. Eight cues per deck and four shared one-shot samples exist; extra sample positions and unavailable FX/controls are labelled coming soon without audio actions. An empty sample pad loads the selected library track; Shift-click replaces a loaded sample with the selection. Sample pads on either deck refer to the same four slots. The pad selection is temporary UI state; switching does not start or stop audio or alter keyboard/MIDI mappings. Clicking a jog toggles playback, the CUE button invokes hot cue 1, and MASTER TEMPO uses curado's key lock; none reproduce full hardware behavior. Curado effects and musical controls sit inside each deck below its pads, with linked transport in A and the keyboard map with action feedback in B. The full curado mixer remains expandable. The console retains its proportions and fits the available sheet width and height; in very small windows it stops shrinking at 65% and allows scrolling. Expanded beatgrid editing or status messages can increase its height. This is not an exact replica; placement, sizing, controls and response still need visual and physical comparison. Switching the layout only changes presentation and does not send a native mode command. No new visual or listening review has been performed.

Layout, visual style and capability are separate: selecting a console changes presentation, the engine determines what works, and the existing keyboard/MIDI policy remains in force. Unsupported reference controls must stay visibly and accessibly unavailable until their function exists. Sample slots remain global one-shots even when controlled from a deck's pads.

Explore a perspective or true 3D version later only after testing whether it adds useful interaction. The existing Metal window is a separate audio-reactive visualization, not a mixer renderer. A 3D interface must retain accessible semantic controls, input and focus handling, be responsive, and remain independent of the callback and transport clock. No renderer, asset format or dependency is selected here.

## Reference and gap matrix

| Reference                                                                      | Hardware characteristics                                                                                                                                          | Current coverage and gaps                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [XDJ-RX3](https://www.pioneerdj.com/en/product/all-in-one-dj-systems/xdj-rx3/) | Two-channel all-in-one with deck jogs and pads, 14 Beat FX, six Sound Color FX and Release FX.                                                                    | Closest starting layout: A/B, EQ, filters, echo, loops, sync and eight hot cues exist. Missing: hardware-specific pad modes, continuous jog/scratch, full FX selection/routing and independent headphone cue. Existing effects are not equivalent emulations.                                                                                                                                                                                                                                                                       |
| [DJM-750MK2](https://www.pioneerdj.com/en/product/dj-mixers/djm-750mk2/)       | Four-channel mixer with Beat FX, Sound Color FX, frequency selection and external send/return; pairs with players.                                                | Only two music channels today. A four-channel skin needs inactive, clearly identified channels until real routing exists; full channel, frequency-selective FX and send/return require engine and device work. [Official FX details](https://www.pioneerdj.com/en/product/features/mixer/pro-fx-djm-750mk2/?product=DJM-750MK2).                                                                                                                                                                                                    |
| [DJM-A9](https://www.pioneerdj.com/en/product/dj-mixers/djm-a9/)               | Four-channel mixer, 14 Beat FX including Mobius and triplet effects, X-Pad, Center Lock, independent dual headphone monitoring, mic effects and external routing. | Current stereo output has no independent headphone bus, much less two. Four-channel sources, FX bus/routing, Center Lock behavior, mic input/effects and external I/O are absent. A visual A9-inspired layout cannot imply these are functional.                                                                                                                                                                                                                                                                                    |
| [CDJ-3000X](https://alphatheta.com/en/product/player/cdj-3000x/black/)         | Separate player with jog, eight hot cues, Touch Cue/Preview and Smart/Gate Cue workflows.                                                                         | Decks have eight hot cues and a clickable platter, with no continuous jog/scratch or independent preview bus. Cue modes and touch monitoring require distinct work; cloud/streaming integration is a separate product and permissions project. [Cue workflows](https://alphatheta.com/en/information/alphathetas-next-generation-professional-dj-player-cdj-3000x/) and [manual settings](https://downloads.support.alphatheta.com/manuals/dj-players/CDJ-3000X/html/en/000_CDJ-3000X_IM_01_EN_DRI1956-A_en/Settings/Settings.htm). |

The linked CDJ is the **3000X**, distinct from the CDJ-3000. These references are research, not approval to reuse their logos, photos, product artwork or proprietary assets. Exact replicas and commercial distribution need a separate brand, design and asset-rights review; initial concepts should use original names and artwork.

## Suggested implementation order

1. Review the new two-deck surface and its proportions, readability, scrolling, themes, keyboard, screen-reader and focus behavior; check switching during playback. Visual app testing requires explicit authorization. Iterate on a closer model-specific arrangement only after the reference and tradeoffs are agreed; do not add fake capabilities.
2. Complete the smooth native position clock priority in [Roadmap](ROADMAP.md). A console cannot solve 20 Hz transport presentation itself.
3. Design multichannel device routing and an independent headphone bus, then verify with real hardware. Specify cue/master mix, device/channel selection, failure recovery and platform support before adding Touch Preview or A9-style dual monitoring.
4. Extend deck interaction and pads incrementally: continuous jog/nudge, explicit scratch behavior and independently specified pad modes. Preserve engine transport ownership and MIDI/keyboard contracts.
5. Design an effects architecture by signal path and capability: channel versus master selection, beat division, dry/wet, bypass, tails, parameter automation and frequency selection. Add individual effects only with bounded preallocated processing, smoothing, latency and denormal review, callback-budget measurements and listening tests. An echo or filter with a familiar name does not establish sonic equivalence.
6. Investigate four full music decks, input sources, mic/booth and external sends as separate work, not as a visual shortcut. Revisit samples, memory, transport, queue, focus, keyboard and MIDI when channel counts change.

## Dedicated audio and hardware backlog

- **Beat FX:** define per-channel/master routing, effect selection, BPM/beat source and divisions, time/depth, enable/bypass and tails. The existing per-deck beat echo is not an RX3 or A9 emulation. Delay, ping pong, reverb, flanger, phaser, roll, slip roll, vinyl brake, helix and the A9 triplet/Mobius effects are not implemented as these hardware modes.
- **Sound Color FX:** design a centered bipolar control and parameter behavior, including A9 Center Lock. Existing separate LP/HP filters are not the hardware's single color knob; sweep, crush, dub echo, noise and space modes remain unimplemented.
- **Mixer path and monitoring:** trim versus channel fader, EQ/isolator response, crossfader curves and assignments, effect ordering and headroom need explicit design and measurement. Independent cue/master routing, dual headphone buses, booth, microphone and external send/return are not available.
- **Player and pads:** scratch/jog modes and touch handling, beat jump, hardware-specific pad modes, release FX, Touch Preview/Cue and Smart/Gate Cue need separate transport, persistence, scheduling and routing designs. Sampler mode controls curado's independent global one-shots, not an RX3 sampler emulation.
- **Quality gates:** define sample-rate-aware smoothing, bypass and tail semantics, latency, denormal handling, bounded work and preallocated buffers for every DSP addition. Measure aliasing, discontinuities, headroom and callback cost, then validate by listening and with hardware; do not claim matching sound without evidence.

Open decisions: how close to individual equipment layouts to go; whether a later four-channel console shows inactive channels; and which functional gaps matter most. No physical controller comparison or listening test has been performed.
