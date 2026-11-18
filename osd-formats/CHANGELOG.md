# Changelog

## Unreleased

- The pencilresearch/midi CSV imports a row with a CC and an NRPN as one control (0.6): the CC is the first way, the NRPN the first of `also`, and the row's single range goes to each way; the control has one default, the CC's. The export writes one row per control, from its first CC way and its first NRPN way, the range the two ways share written once in the CC columns, so the round trip CSV, OSD, CSV of such a row loses nothing. A way a row cannot carry (a second CC or NRPN, an RPN) is a loss at its pointer `/cc_params/entries/N/also/M`; the round trip OSD, CSV, OSD is not promised for it. A way whose message an earlier row (or the base of `import_into`) already claims is left out and named as a loss at its cell, and a row left with no way is lost, so the imported descriptor reads in 0.6. The Electra One export names `also` as a loss: a preset binds one message to a control.
