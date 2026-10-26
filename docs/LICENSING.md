# Licensing and distribution decisions

**Status: no project license selected.** This document compares options; it does not grant a license or replace a `LICENSE` file. The maintainer wants broad freedom, including commercial forks, and is considering free downloads, donations and paid editions. The recommendation below reflects that preference.

## Recommendation: MIT, with brand rules decided separately

MIT is a good fit if curado should be easy to use, modify, redistribute and include in commercial work. It requires preserving copyright/license notices and allows others to distribute closed modifications. It does not require forks to send changes back. See the [MIT text and summary](https://choosealicense.com/licenses/mit/).

The same permissions apply to other people: they could sell their own curado-based build. An open license cannot promise the author an exclusive market for the code it grants others permission to use. Keep the official release identity clear through an independently decided name/logo policy; no trademark registration or brand policy is asserted here.

Apache-2.0 is the other permissive option worth considering if an explicit contributor patent grant matters. It adds patent provisions, notice/change requirements and does not grant trademark rights. It is longer than MIT but still permits commercial and closed derivatives. [Apache-2.0 text](https://www.apache.org/licenses/LICENSE-2.0) and [summary](https://choosealicense.com/licenses/apache-2.0/).

## Free downloads, donations and paid versions

These distribution models can coexist under a permissive license:

| Model                                               | What you can offer                                    | Practical implication                                                                                       |
| --------------------------------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Free official build                                 | A download without payment                            | Still needs signing, notices, quality evidence and release infrastructure                                   |
| Donations / pay what you want                       | Optional support for development                      | Payment does not change code permissions or create exclusivity                                              |
| Paid official build                                 | Convenience, packaging, updates or a polished release | Others may also build and distribute the licensed code                                                      |
| Paid support                                        | Installation help, support or services                | Define service terms separately from the code license                                                       |
| A paid edition with additional proprietary features | Separately owned code distributed on different terms  | Audit dependency and contributor rights; code already released under MIT remains available under that grant |

Charging is not incompatible with open source. GPL also allows charging; its distinction is source-sharing obligations on covered distribution, not a ban on sales. [GNU on selling free software](https://www.gnu.org/philosophy/selling.html).

For curado, a straightforward starting model would be **MIT source + free local builds + optional donations**, leaving a paid official edition/support as a later product choice. No price, payment provider, paid feature boundary or release promise has been chosen. MIT grants already made cannot simply be revoked from existing recipients by switching a future release to different terms.

## Alternatives if your priorities change

| Option                                        | Main tradeoff                                                                                              | Fit for the stated preference                                                 |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| MIT                                           | Very simple; commercial and closed forks allowed                                                           | Recommended for broad reuse                                                   |
| Apache-2.0                                    | Permissive plus express patent provisions and more notice requirements                                     | Strong alternative if patents matter                                          |
| MPL-2.0                                       | Distributed modifications to covered files remain available as source; separate files can use other terms  | Consider if sharing changes to curado files matters more than maximum freedom |
| GPL-3.0                                       | Broader copyleft/source obligations when conveying a covered combined work; commercial sales still allowed | Consider for reciprocal openness, not for preventing commercial use           |
| Proprietary or a custom noncommercial license | Greater restrictions; not the same open-source permission model                                            | Does not match commercial-fork freedom                                        |

[MPL FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) explains its file-level scope; [GPL-3.0](https://www.gnu.org/licenses/gpl-3.0.html) defines its conditions. Do not add “no commercial use” to MIT and continue calling the result MIT. Dual licensing requires sufficient rights from all relevant owners; choosing it later is not a substitute for managing contributions now.

## What is unresolved in this repository

1. **Project license and holder:** confirm MIT or another choice, the copyright holder's actual credited identity and year. `nu9ve.xyz` is an author link, not evidence of legal entity/ownership.
2. **Contribution terms:** decide how outside contributions are accepted and whether future relicensing/proprietary editions need additional permissions. A DCO or CLA is a separate choice, not currently installed policy.
3. **Brand and assets:** define permission for curado's name/logo, screenshot and original demo material separately if desired. A code license should not accidentally imply rights to third-party music or provider branding.
4. **Dependency obligations:** complete the license inventory for the locked Rust/JS trees and shipped assets, keep required texts/notices and provide covered source where required.
5. **Paid distribution:** choose product boundaries, pricing, support/refund terms, signing/notarization and the actual download/update channel before announcing a paid edition.

There is no top-level `LICENSE` and neither Rust package currently declares a license field. `package.json` is private and has no project license declaration. Do not add an SPDX identifier or an “open source” badge before the maintainer applies a license. Public visibility alone is not general permission to reuse code; platform terms can grant narrower rights. [No-license explanation](https://choosealicense.com/no-permission/).

## Existing third-party material

This is a starting inventory, **not a completed compatibility audit**:

| Material                                                           | Evidence in this checkout                                                       | Remaining release work                                                                                                                    |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Signalsmith Rust wrapper, Stretch and Linear                       | MIT notices in `licenses/signalsmith-stretch/`; bundled through Tauri resources | Retain notices and verify against actual shipped versions                                                                                 |
| Symphonia 0.5.5                                                    | Resolved crate metadata declares MPL-2.0                                        | Include required notice and tell recipients how to obtain exact covered source, including any modifications; review all enabled subcrates |
| Jersey 15, Pixelify Sans, Geist Mono, UnifrakturMaguntia, Michroma | Fontsource 5.3.0 packages each include their font license                       | Preserve the exact family notices when distributing bundled fonts; docs currently identify them as OFL 1.1                                |
| Other Rust/JS dependencies, icons and generated/third-party assets | `Cargo.lock`, `bun.lock`, package metadata and source notices                   | Full locked-tree inventory, compatibility review and assembled release notices remain pending                                             |
| YouTube/SoundCloud/fractos content                                 | Embedded remote services, no grant of media rights through curado               | Provider terms/media rights remain separate from the app's code license                                                                   |

Using an MPL dependency does not automatically require licensing every separate curado source file as MPL. It does create obligations for covered files when distributing them, including in executable form. Mozilla's [FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) explains source availability and notices. Do not label an entire bundle “MIT only” or assume dependency source obligations disappear because the app is sold or given away.

Existing bundled notices do not yet cover the whole dependency/font inventory; the current build is not a licensing-complete public release.

## Applying the decision later

Once explicitly chosen: add the exact license text with confirmed holder/year, align both Rust package metadata and JS metadata, update README statements, decide asset/brand/contribution terms, assemble third-party notices/source instructions, and check the actual distributable. Record the choice in DECISIONS and STATUS. This documentation session intentionally does not perform those licensing changes.
