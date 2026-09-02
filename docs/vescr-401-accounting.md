# VESCR-401 source accounting

This manifest is generated from the immutable source reference
`mjc/fob-refloat-main-port-and-10k@eb424686`. Source positions are
one-based entries in `git rev-list --reverse origin/main..eb424686`; the
ledger below therefore accounts for all 194 source commits.

## Area ledger

Every corrected local area is stacked in the order shown. Additions plus
deletions are measured against that area's actual base. The largest area is
4,643 changed lines.

| Area | Local branch | Base → tip | Commits | Reviewable delta | Source ownership |
| --- | --- | --- | ---: | --- | --- |
| firmware-700-sdk | `mjc/vescr-395-firmware-700-sdk` | `origin/main` → `2374aeb8` | 7 | +1,003 / -547 (1550 changed) | source positions 1-6 and 11 |
| led-lcm-ws2812 | `mjc/vescr-401-led-lcm-ws2812` | `2374aeb8` → `515b8d1b` | 6 | +1,425 / -1,237 (2662 changed) | source positions 7-9; characterization hunks from 15 |
| config-fields | `mjc/vescr-401-config-fields` | `515b8d1b` → `8019c717` | 5 | +221 / -682 (903 changed) | source position 10 |
| payload-fields | `mjc/vescr-401-payload-fields` | `8019c717` → `c09eda81` | 5 | +301 / -1,027 (1328 changed) | source position 12 |
| runtime-handoffs | `mjc/vescr-401-runtime-handoffs` | `c09eda81` → `7e13429d` | 3 | +618 / -1,024 (1642 changed) | source positions 13-14 |
| tests-balance-config | `mjc/vescr-401-tests-balance-config` | `7e13429d` → `15acb4f6` | 2 | +2,308 / -2,335 (4643 changed) | source position 15 balance/sensor/config/hardware hunks |
| tests-led | `mjc/vescr-401-tests-led` | `15acb4f6` → `747ad9df` | 1 | +1,564 / -1,573 (3137 changed) | source position 15 LED renderer hunks |
| tests-protocol-safety | `mjc/vescr-401-tests-protocol-safety` | `747ad9df` → `dce53545` | 2 | +1,985 / -2,013 (3998 changed) | source position 15 protocol/state-safety hunks |
| tests-runtime-state | `mjc/vescr-401-tests-runtime-state` | `dce53545` → `a46fc2f0` | 1 | +1,740 / -1,755 (3495 changed) | source position 15 runtime-state hunks |
| tests-thread-loc | `mjc/vescr-401-tests-thread-loc` | `a46fc2f0` → `4dab5254` | 1 | +1,176 / -751 (1927 changed) | source position 15 thread/LOC-tooling hunks |
| led-config-access | `mjc/vescr-401-led-config-access-split` | `4dab5254` → `9f41d19d` | 1 | +626 / -640 (1266 changed) | source position 16 |
| led-rendering | `mjc/vescr-401-led-rendering-split` | `9f41d19d` → `714f2c99` | 1 | +156 / -280 (436 changed) | source position 17 |
| generated-abstractions | `mjc/vescr-401-generated-abstractions-split` | `714f2c99` → `9b46a177` | 17 | +1,454 / -2,978 (4432 changed) | source positions 18-34 |
| runtime-wire | `mjc/vescr-401-runtime-wire-split` | `9b46a177` → `d1b0084f` | 14 | +1,572 / -3,026 (4598 changed) | source positions 35-48 |
| protocol-sdk | `mjc/vescr-401-protocol-sdk-split` | `d1b0084f` → `a5db9fbb` | 11 | +2,486 / -1,958 (4444 changed) | source positions 49-59 |
| types-and-rings | `mjc/vescr-401-types-and-rings-split` | `a5db9fbb` → `4f1b0b22` | 20 | +1,802 / -2,055 (3857 changed) | source positions 60-79 |
| 10k-reduction | `mjc/vescr-401-10k-reduction-split` | `4f1b0b22` → `7d82122b` | 17 | +1,972 / -2,436 (4408 changed) | source positions 80-96 |
| legacy-behavior | `mjc/vescr-401-legacy-behavior-split` | `7d82122b` → `b9112cc2` | 16 | +3,269 / -1,130 (4399 changed) | source positions 97-112 |
| legacy-port-2 | `mjc/vescr-401-legacy-port-2-split` | `b9112cc2` → `9616e0d8` | 9 | +3,243 / -1,276 (4519 changed) | source positions 113-121 |
| cutoff-docs | `mjc/vescr-401-cutoff-docs-split` | `9616e0d8` → `7a566f8a` | 20 | +2,837 / -723 (3560 changed) | source positions 122-141 |
| docs-arm | `mjc/vescr-401-docs-arm-split` | `7a566f8a` → `faaa39cb` | 12 | +406 / -26 (432 changed) | source positions 142-153 |
| runtime-locks | `mjc/vescr-401-runtime-locks-split` | `faaa39cb` → `a2fa10f1` | 15 | +1,309 / -682 (1991 changed) | source positions 154-168 |
| snapshot-persistence | `mjc/vescr-401-snapshot-persistence-split` | `a2fa10f1` → `5c4d670e` | 5 | +1,711 / -1,206 (2917 changed) | source positions 169-173 |
| shared-runtime | `mjc/vescr-401-shared-runtime-split` | `5c4d670e` → `2e48628c` | 11 | +2,492 / -1,422 (3914 changed) | source positions 174-184 |
| final-typed-sdk | `mjc/vescr-401-final-typed-sdk-split` | `2e48628c` → `e152cd4a` | 10 | +1,500 / -1,480 (2980 changed) | source positions 185-194 |

The local final tip is tree-equivalent to `eb424686`; the local commits are
historical, GPG-signed reconstructions rather than a claim that each local
commit has the same object ID as its source commit.

The one- and two-commit areas are intentionally atomic source/hunk groups;
splitting those further would create mechanical commits without a useful
review boundary. The larger areas retain 5–20 meaningful commits.

## Cross-area hunk split for source commit 15 (`84cee8ac`)

`84cee8ac` is one large test-relocation commit. Its hunks are intentionally
assigned by ownership, with same-file hunks explicitly allowed to land in
different areas:

| Destination | Hunk/file ownership |
| --- | --- |
| led-lcm-ws2812 | Characterization hunks in `examples/float-out-boy/src/lcm/hardware.rs`, `src/leds.rs`, `src/package/state/internal_leds/driver.rs`, and `src/package/state/lcm.rs`. |
| tests-balance-config | Balance/filter/step, beeper, BMS, domain app-data, extensions, motor-control, config, and LCM/internal-LED hardware test moves. |
| tests-led | LED renderer extraction from `src/leds.rs` into `src/leds/renderer_tests.rs`. |
| tests-protocol-safety | Package callbacks, custom config, IMU callback, protocol metadata/realtime/wire, startup, package inventory, and state safety test moves. |
| tests-runtime-state | LCM state, packet responses, remote control, ride modifiers, transitions, and tuning test moves. |
| tests-thread-loc | Thread/time/wire test moves, package test support, top-level test inventory, `Makefile`, `tools/fob-production-rust`, and `tools/fob-production-tokei.sh`. |

## Complete source ledger

| Position | Source commit | Author date | Subject | Destination |
| ---: | --- | --- | --- | --- |
| 1 | `a8f8577ceb50c0fa086f954d5e0e6926a899564b` | 2026-07-30T17:02:55-06:00 | fix(sdk): enforce firmware command boundaries | firmware-700-sdk |
| 2 | `c3d1f1c438c8d2b9f54312fe401cb46a8a82bd67` | 2026-07-30T17:10:01-06:00 | refactor(sdk): make pedantic lint scope explicit | firmware-700-sdk |
| 3 | `b873a5fdfdce4e8f1adc69d9c02641cadc954a79` | 2026-07-30T17:12:55-06:00 | refactor(sdk): trim motor forwarding layers | firmware-700-sdk |
| 4 | `ef5227a7013621d13fc4b552754426e70fe04cd3` | 2026-07-30T17:37:30-06:00 | Report device firmware in values probe | firmware-700-sdk |
| 5 | `8017cf3806dd0893845a3bd0c2014d5287f24c9a` | 2026-07-30T17:37:38-06:00 | Advance STM32 package ABI to firmware 7.00 | firmware-700-sdk |
| 6 | `28806a4354acfc9eb98418c7cb601d20db7baef1` | 2026-07-30T17:37:44-06:00 | Expose firmware 7 motor semantics | firmware-700-sdk |
| 7 | `1aabde2ab383389fe9b9714a06d771412b0c2043` | 2026-08-02T14:33:27-06:00 | refactor(float-out-boy): reduce LED and LCM boilerplate | led-lcm-ws2812 |
| 8 | `70f8f121e035ed3b503d160e2cb91c6860677267` | 2026-08-02T15:37:32-06:00 | refactor(float-out-boy): extract provisional WS2812 support | led-lcm-ws2812 |
| 9 | `c59dc3373eee65c3db52f51455e8670f8dcdbb3d` | 2026-08-02T16:42:03-06:00 | refactor(float-out-boy): simplify LED and LCM paths | led-lcm-ws2812 |
| 10 | `5734eebd0767d44ab5c9f3715716b93f9368f88e` | 2026-08-02T17:33:24-06:00 | refactor(float-out-boy): collapse generated config fields | config-fields |
| 11 | `ea52366f113aa809804598e3ef3c34e917a983d2` | 2026-08-02T17:41:05-06:00 | fix(sdk): restore complete check matrix | firmware-700-sdk |
| 12 | `e038a35fc740a6166e2edad3a5928feadb299cde` | 2026-08-02T18:06:31-06:00 | refactor(float-out-boy): collapse payload field boilerplate | payload-fields |
| 13 | `2e62c99dc183f37b6b726380acf354756dca765d` | 2026-08-02T18:33:17-06:00 | refactor(float-out-boy): collapse runtime handoffs | runtime-handoffs |
| 14 | `c6d411dcb8c9484a10f9a22e5ef24ee2630afa90` | 2026-08-02T18:43:05-06:00 | refactor(float-out-boy): collapse running control pipeline | runtime-handoffs |
| 15 | `84cee8ac0e6d91a693a9361f4fdd767884655c3b` | 2026-08-02T19:27:34-06:00 | test(float-out-boy): separate production LOC | led-lcm-ws2812 [characterization]; tests-balance-config [hunks]; tests-led [hunks]; tests-protocol-safety [hunks]; tests-runtime-state [hunks]; tests-thread-loc [hunks] |
| 16 | `5b6ddb32e63bd8bf0830fbd579358df3e1cc58ac` | 2026-08-02T19:54:49-06:00 | refactor(float-out-boy): flatten LED config access | led-config-access |
| 17 | `a57b7ca8e9727793830c69e11071a935706222a7` | 2026-08-03T09:27:16-06:00 | refactor(float-out-boy): collapse LED decode and rendering | led-rendering |
| 18 | `d2107a4d91cbabc5af5c570413a6443a19596c29` | 2026-08-03T09:34:08-06:00 | refactor(float-out-boy): generate realtime item metadata | generated-abstractions |
| 19 | `89bdc77c8a95b38d730be6dda50eb7f3dd6c6a0b` | 2026-08-03T09:44:51-06:00 | refactor(float-out-boy): generate state wire enums | generated-abstractions |
| 20 | `fbb08944bee7fa5b5e80ad8cb7da5e782e33773e` | 2026-08-03T09:53:38-06:00 | refactor(float-out-boy): generate config accessors | generated-abstractions |
| 21 | `5c8b4bdee739f24f80353e890a18461c5ca3dca3` | 2026-08-03T10:22:54-06:00 | refactor(float-out-boy): unify packet writers | generated-abstractions |
| 22 | `f4b2dc60ecba4c8be4ef3d62a9479e78ff1b6f46` | 2026-08-03T10:30:50-06:00 | refactor(float-out-boy): collapse config tune boilerplate | generated-abstractions |
| 23 | `2eededbfce48f219c8f75b9e802e7213527140d3` | 2026-08-03T10:47:25-06:00 | refactor(float-out-boy): share declaration macros | generated-abstractions |
| 24 | `7a4025fc6a90f5c7ca95b803801ad586c0b9367d` | 2026-08-03T11:03:39-06:00 | refactor(float-out-boy): unify generated flag fields | generated-abstractions |
| 25 | `e041a93f3c8459e4102fd65f1bded32f6400e632` | 2026-08-03T11:11:33-06:00 | refactor(float-out-boy): simplify data recorder ring | generated-abstractions |
| 26 | `dec05f27842452c44113403e0b5142d450764d73` | 2026-08-03T11:18:15-06:00 | refactor(float-out-boy): collapse recorder request state | generated-abstractions |
| 27 | `9e6ae526b9185e8b0b97e6b63a3574928dc7159f` | 2026-08-03T11:21:37-06:00 | refactor(float-out-boy): flatten beeper counters | generated-abstractions |
| 28 | `e7a55742ee57cb8a23cef5c913fac6b87fcec19a` | 2026-08-03T11:35:56-06:00 | refactor(float-out-boy): share fixed packet buffer | generated-abstractions |
| 29 | `78fd675e83abd4a82aed084c25286d2e4ad0d507` | 2026-08-03T11:45:53-06:00 | refactor(float-out-boy): delete realtime metadata wrappers | generated-abstractions |
| 30 | `0ca22a5815e60749b7111cc126448045a2dbbe7c` | 2026-08-03T11:51:01-06:00 | refactor(float-out-boy): delete unused realtime payload models | generated-abstractions |
| 31 | `0bd5ffa9e1a58d62e4fd35d65c3ad8589cb4bf6f` | 2026-08-03T13:22:39-06:00 | refactor(float-out-boy): delete test-shaped LED layout model | generated-abstractions |
| 32 | `674965b8fdc6fa9444b89166e2a4cc412512daed` | 2026-08-03T13:26:54-06:00 | refactor(float-out-boy): delete redundant realtime wrappers | generated-abstractions |
| 33 | `28e41f5638bd20feb8b0263a18da90ece51de70b` | 2026-08-03T13:29:59-06:00 | refactor(float-out-boy): return LED channels directly | generated-abstractions |
| 34 | `38cff4f5339ef9bffd7cb85a20091e302919ff49` | 2026-08-03T13:35:24-06:00 | refactor(float-out-boy): store requested motor current directly | generated-abstractions |
| 35 | `204c55684f779f7df38dfc1c316b5cae04f3e15e` | 2026-08-03T13:41:27-06:00 | refactor(float-out-boy): use the app-data package id directly | runtime-wire |
| 36 | `71918368f722ce830ffe48ff7ff63b60d0916c2f` | 2026-08-03T13:54:55-06:00 | refactor(float-out-boy): flatten LED frame inputs | runtime-wire |
| 37 | `7ddc7d65f80e79391560720c2fddbae4a33fa721` | 2026-08-03T14:04:24-06:00 | refactor(float-out-boy): flatten IMU runtime handoffs | runtime-wire |
| 38 | `dd8c74dd2175fb86830f3d33070fb1b225496c5a` | 2026-08-03T14:20:04-06:00 | measure FOB production for ARM | runtime-wire |
| 39 | `04def64f2209210a82353724fce60e8617fe5e40` | 2026-08-03T14:27:28-06:00 | simplify FOB beeper counts | runtime-wire |
| 40 | `5521b62fd554dacb369f20b7c7e9e2cc54b210ef` | 2026-08-03T21:19:04-06:00 | refactor(float-out-boy): share typed wire defaults | runtime-wire |
| 41 | `d4057b1b0bc8b7ad809a83449eaaa84bcfb0a531` | 2026-08-03T21:29:26-06:00 | refactor(float-out-boy): generate field accessors | runtime-wire |
| 42 | `c119614f4756dca1f22a51d5551f1d53192d7046` | 2026-08-03T21:58:53-06:00 | refactor(float-out-boy): derive source startup state | runtime-wire |
| 43 | `415ab3b8264b317837468ec3729683cd48cec069` | 2026-08-03T22:11:42-06:00 | refactor(float-out-boy): generate BMS field groups | runtime-wire |
| 44 | `511ad029200e4ef4a618d8be8fdec057f7c471d7` | 2026-08-03T22:25:55-06:00 | refactor(float-out-boy): use typed runtime flags | runtime-wire |
| 45 | `1648ac74ec1a3e6a2e984c9d6f5a9694061c6cf2` | 2026-08-03T22:43:29-06:00 | refactor(float-out-boy): collapse balance filter algebra | runtime-wire |
| 46 | `d11b7e5d9aefcaaf8f7ad20ffa847d0d8b9d7880` | 2026-08-03T22:52:47-06:00 | refactor(float-out-boy): flatten PID scale flow | runtime-wire |
| 47 | `06ce51649fc4d2a532d76faccc13330121da3b97` | 2026-08-03T22:58:29-06:00 | refactor(float-out-boy): flatten booster flow | runtime-wire |
| 48 | `ffa295d7bdfa2dcd076d85c0e3e74e7f692d9532` | 2026-08-03T23:09:04-06:00 | refactor(float-out-boy): reuse LED render state | runtime-wire |
| 49 | `a5ff8ca4bb87095a53cf368dc073b308c888e0ee` | 2026-08-03T23:50:31-06:00 | refactor(float-out-boy): isolate FOB protocol and LEDs | protocol-sdk |
| 50 | `66e26d76cf6cca0ebed49ed84f7a431d04edfbf8` | 2026-08-03T23:57:48-06:00 | test(float-out-boy): exclude LED support APIs from production | protocol-sdk |
| 51 | `0aefaecd8691a42ce6e0532297a3b7f315904a01` | 2026-08-04T00:20:12-06:00 | refactor(float-out-boy): share EEPROM image commits | protocol-sdk |
| 52 | `afe89f73f73dedf0e93d87e150a67d988c572807` | 2026-08-04T00:44:22-06:00 | refactor(float-out-boy): generate typed config setters | protocol-sdk |
| 53 | `acd67fe2e78212ff7518035f3e90c8aeaf66f5bf` | 2026-08-04T01:05:31-06:00 | refactor(float-out-boy): share compact wire decoding | protocol-sdk |
| 54 | `dd16761f61c5065b9d35ead78680a73818b69f04` | 2026-08-04T01:16:52-06:00 | refactor(vescpkg): share recorder ring cursor | protocol-sdk |
| 55 | `9b9a70f03c348392a31e3ee62b83ed5b010b4b16` | 2026-08-04T01:35:30-06:00 | refactor(vescpkg): generalize STM32 DMA PWM lifecycle | protocol-sdk |
| 56 | `6b7917b2f8969dd40d3eac2dee3dc80aed2e9ad5` | 2026-08-04T01:50:01-06:00 | refactor(float-out-boy): simplify runtime thread descriptors | protocol-sdk |
| 57 | `31528602e20e2c8c9b607f93afdc04a5cc16c803` | 2026-08-04T02:03:22-06:00 | fix(vescpkg): enable stateful helpers in unit tests | protocol-sdk |
| 58 | `a25218e22145e4822dff9727733dc6f4a53f67c7` | 2026-08-04T02:03:29-06:00 | refactor(vescpkg): share typed slew limiting | protocol-sdk |
| 59 | `f6ed3d17b0589fc9a77b697ea7bff477bd7d904d` | 2026-08-04T02:11:49-06:00 | refactor(vescpkg): own firmware fault history | protocol-sdk |
| 60 | `98be4ccfe29efd5fe841ad9897edcf68a7683289` | 2026-08-04T02:22:36-06:00 | refactor(vescpkg): own fixed record rings | types-and-rings |
| 61 | `e080a851975eeec502eaa2a9f1c945c039a14a77` | 2026-08-04T02:33:46-06:00 | refactor(float-out-boy): unify all-data response storage | types-and-rings |
| 62 | `059ef6ee0de4397dc1da6389ace92eb2359ae554` | 2026-08-04T02:40:09-06:00 | refactor(float-out-boy): inline remote current smoothing | types-and-rings |
| 63 | `44a50521dcf43e6fb837e1d4774e9ec2a8b08167` | 2026-08-04T02:48:26-06:00 | refactor(protocol): own VESC float16 encoding | types-and-rings |
| 64 | `98c6907b4b320005bf671c1d185d6d051562bc44` | 2026-08-04T02:58:35-06:00 | refactor(float-out-boy): trim test-only protocol helpers | types-and-rings |
| 65 | `8d7c7f34b9b4474a8bbbfa9af63d09bee3cc1f71` | 2026-08-04T03:04:30-06:00 | refactor(float-out-boy): trim local runtime delegators | types-and-rings |
| 66 | `4fc30a2f674d8e697fa9e35bfaca39681a69b860` | 2026-08-04T03:13:30-06:00 | refactor(float-out-boy): collapse balance current adapters | types-and-rings |
| 67 | `746291b51e0e96ded74c8be2f36d0a7f64cccebc` | 2026-08-04T03:18:15-06:00 | refactor(float-out-boy): inline booster adapters | types-and-rings |
| 68 | `fac77b80174ab49d533f0eab0f9e6d2f83c00114` | 2026-08-04T03:23:18-06:00 | refactor(protocol): use generated command conversion | types-and-rings |
| 69 | `a31491d8b754d4923090f3b0b1f06f77ae331cc5` | 2026-08-04T03:37:45-06:00 | refactor(float-out-boy): declare tuning field writes | types-and-rings |
| 70 | `bf3380997a20f1f32d784dfb3c029a265de05c5d` | 2026-08-04T03:58:33-06:00 | refactor(float-out-boy): collapse config and fault decoding | types-and-rings |
| 71 | `527aa55bdfd9d10f3690cc77897b8a934fb84cbb` | 2026-08-04T04:08:33-06:00 | refactor(float-out-boy): share typed angle ramp | types-and-rings |
| 72 | `c3517a060fbd24554e33c2c5722ce249ddf39511` | 2026-08-04T04:30:43-06:00 | refactor(float-out-boy): share fallible allocation owner | types-and-rings |
| 73 | `964dd0513bb596e5e4b5af42ca5fe31e162b25a9` | 2026-08-04T04:45:48-06:00 | refactor(float-out-boy): share fixed ring cursor | types-and-rings |
| 74 | `32f8ae844b2b62c3dcd26f0af8170bae1b6d801e` | 2026-08-04T04:53:02-06:00 | refactor(float-out-boy): single-source LED palette | types-and-rings |
| 75 | `3d88ddeed6bdb1217bc92213195ad8b4d661be7f` | 2026-08-04T05:05:53-06:00 | refactor(float-out-boy): share wrapping timer operations | types-and-rings |
| 76 | `85659109d8cf93305ddff56c11b87769ae6fe9b0` | 2026-08-04T05:19:38-06:00 | refactor(float-out-boy): type wrapping timers | types-and-rings |
| 77 | `7861e8626c9623cb542290740b2ee0529db38857` | 2026-08-04T05:28:08-06:00 | refactor(float-out-boy): keep LED ratios typed | types-and-rings |
| 78 | `516950bbc53bfee735f3ce9d6cd95322eb79e3d4` | 2026-08-04T05:35:45-06:00 | refactor(float-out-boy): single-source realtime projections | types-and-rings |
| 79 | `2e2b85a5050417a23799c986c93f41ed97a0b80b` | 2026-08-04T05:47:05-06:00 | refactor(float-out-boy): centralize DMA pulse storage | types-and-rings |
| 80 | `aba7dbd7ab05a5b2be37e95a28d3d23455e8ea6d` | 2026-08-04T05:53:48-06:00 | refactor(float-out-boy): collapse state transition dispatch | 10k-reduction |
| 81 | `cd89c085bd586424d7f10c990eff0fb0b11ce3ae` | 2026-08-04T06:01:04-06:00 | refactor(float-out-boy): extract motor kinematics | 10k-reduction |
| 82 | `9a2926efb0294ad12d591f1d1ab2901cdf033f3b` | 2026-08-04T06:06:48-06:00 | refactor(float-out-boy): update one runtime payload | 10k-reduction |
| 83 | `8a602d643e932eeedc04aeeca3587c9f892776b7` | 2026-08-04T06:16:17-06:00 | refactor(float-out-boy): extract wrapped angle motion | 10k-reduction |
| 84 | `ef17e280f1291b53cc1b2a9557eb6d6e32289ab2` | 2026-08-04T06:27:04-06:00 | refactor(float-out-boy): extract biquad low-pass filter | 10k-reduction |
| 85 | `f6421edb0148171a54b7f9aea0f6ffeab10fb1fe` | 2026-08-04T06:38:28-06:00 | refactor(float-out-boy): extract axis Mahony filter | 10k-reduction |
| 86 | `ee9e835e45e6cd8cdd654d17748371cf5aaf92f3` | 2026-08-04T06:48:30-06:00 | refactor(float-out-boy): extract input tilt slew | 10k-reduction |
| 87 | `4958105b242c51f278cd86cfc695ed90bdc14d82` | 2026-08-04T07:11:14-06:00 | refactor(float-out-boy): share generated config views | 10k-reduction |
| 88 | `62dbcaf85e37b72fe95e97fe219ebb21fbd80dac` | 2026-08-04T07:18:20-06:00 | refactor(float-out-boy): flatten fixed runtime limits | 10k-reduction |
| 89 | `fe7c2b2025837b39918dddb31d38475453ed9be9` | 2026-08-04T07:22:46-06:00 | refactor(float-out-boy): gate firmware modules once | 10k-reduction |
| 90 | `9c356feeaf0ba094855310f0545c53608f727c2f` | 2026-08-04T07:32:43-06:00 | refactor(float-out-boy): share deciamp current unit | 10k-reduction |
| 91 | `5b4ed31627d28afd985d159ed9b5ca3573585d8e` | 2026-08-04T07:40:04-06:00 | refactor(float-out-boy): generate mapped config fields | 10k-reduction |
| 92 | `0a9a1ac025d6b05a8f8cf48d525ee7530e23de34` | 2026-08-04T07:51:59-06:00 | refactor(float-out-boy): share symmetric LED math | 10k-reduction |
| 93 | `b666da5afe7730488addf000cbaafad9a093a08e` | 2026-08-04T07:52:02-06:00 | refactor(float-out-boy): gate firmware source once | 10k-reduction |
| 94 | `436d2fb72d953b27e29f536d2f6b25934e9b3b32` | 2026-08-04T08:03:09-06:00 | refactor(float-out-boy): compact forwarded state | 10k-reduction |
| 95 | `07cd96bed7b793148f748cbcdc159e60118a5ac5` | 2026-08-04T08:18:49-06:00 | refactor(float-out-boy): extract app-data primitives | 10k-reduction |
| 96 | `15724dc963d48646136cc3544eb735429e9f9e55` | 2026-08-04T08:50:53-06:00 | refactor(float-out-boy): reach 10k production lines | 10k-reduction |
| 97 | `ee784a3648415f5507e9065e23e036b5367ef4ca` | 2026-07-29T18:07:41-06:00 | Use elapsed time for Float Out Boy beeps | legacy-behavior |
| 98 | `6e7414c44d46c67756590caa713e8d0b2722c200` | 2026-07-29T18:12:03-06:00 | Initialize LEDs with pressed footpads | legacy-behavior |
| 99 | `dc5fed8a748586289b260b97afc81e2070f5394a` | 2026-07-29T18:24:01-06:00 | Show current saturation on LED status bars | legacy-behavior |
| 100 | `6e97815fe134979f5cb8380857710803a8154505` | 2026-07-29T18:32:32-06:00 | Compensate Float Out Boy loop timing | legacy-behavior |
| 101 | `d75added89612e940ad99ce07cc801f9ba08a767` | 2026-07-29T18:49:53-06:00 | Use measured time in Float Out Boy control | legacy-behavior |
| 102 | `a127fd58f48a95b84ff98f96611877de007f3f41` | 2026-07-29T19:01:32-06:00 | Keep Float Out Boy motor current filtered | legacy-behavior |
| 103 | `4317952f70256353a47ac70b33d33745eb652db0` | 2026-07-29T19:07:19-06:00 | Make Float Out Boy motor data time independent | legacy-behavior |
| 104 | `ed3edab3ce47cc941e63850d48a534274c1db55d` | 2026-07-29T19:14:02-06:00 | Make Float Out Boy balance filters time independent | legacy-behavior |
| 105 | `f84c0c8671cb0a736434c0e4a65d24dabc24d7ab` | 2026-07-29T19:34:34-06:00 | Use distance for Float Out Boy reverse stop | legacy-behavior |
| 106 | `c1d306f3e8a8cb15ee2a9659a101b49ebaa13018` | 2026-07-29T19:42:59-06:00 | Make Float Out Boy turn tilt time independent | legacy-behavior |
| 107 | `39e5d8e2d37765a167617637b349cf3af6f10ff9` | 2026-07-29T19:57:59-06:00 | Run Float Out Boy PID from IMU samples | legacy-behavior |
| 108 | `530e9156dce4e8d051c6f6f55d6057886ad2464a` | 2026-07-29T20:04:21-06:00 | Fix Float Out Boy main loop at 500 Hz | legacy-behavior |
| 109 | `baf40bbf066595abc348c145a707911ebe37e8e4` | 2026-07-29T20:21:22-06:00 | Use smooth setpoints for Float Out Boy turn tilt | legacy-behavior |
| 110 | `a8fd6c3b21c8ec304491a48d163996f0bbd0828c` | 2026-07-29T20:25:50-06:00 | Use smooth setpoints for Float Out Boy remote tilt | legacy-behavior |
| 111 | `d98e25d5e10f7687f8bff4c4fbbb3039775cb1ae` | 2026-07-29T20:38:51-06:00 | Smooth Float Out Boy ride modifiers | legacy-behavior |
| 112 | `40abcae672a9a2291e4f93b3226149ba6ca507ff` | 2026-07-29T20:48:10-06:00 | Use motor torque for Float Out Boy ride modifiers | legacy-behavior |
| 113 | `6c8255138785b39d7f8c52a1cbb9b0add934429f` | 2026-07-29T20:59:06-06:00 | Run Float Out Boy balance control in torque | legacy-port-2 |
| 114 | `d47c0df623352bf6663bc2028f2b8d930acdf12c` | 2026-07-29T21:08:01-06:00 | Report Float Out Boy booster torque | legacy-port-2 |
| 115 | `f5cf2fa52309a8129485432f67c6b972794e6e00` | 2026-07-29T21:32:48-06:00 | Port Float Out Boy cutoff config schema | legacy-port-2 |
| 116 | `05fc3b0a5684bd8655d028b6f140401dd14403de` | 2026-07-29T21:47:28-06:00 | Port Float Out Boy internal realtime data | legacy-port-2 |
| 117 | `7bf6f84f77f34f714ba152e624d14d5da36b66b2` | 2026-07-29T22:06:49-06:00 | Port Float Out Boy selected realtime data | legacy-port-2 |
| 118 | `f6fa03385bbc20f995f19cd9e62d4e3b4363766f` | 2026-07-29T22:08:54-06:00 | Version the firmware data recorder descriptor | legacy-port-2 |
| 119 | `2a9c9d34c42fe897125f184a33cab1b59fffefe0` | 2026-07-29T22:14:21-06:00 | Port Float Out Boy recorder controls | legacy-port-2 |
| 120 | `8137dd867e472abd43c5cb626df8d8875a10770b` | 2026-07-29T22:18:11-06:00 | Preserve Float Out Boy config identity | legacy-port-2 |
| 121 | `cf37785b5fc91ac070127cb4a731e288602df588` | 2026-07-29T22:36:32-06:00 | Port unified Float Out Boy remote control | legacy-port-2 |
| 122 | `ce638be0c0a69ef83f73b594a012840a67846a0f` | 2026-07-29T22:45:19-06:00 | Port cutoff footpad ADC mapping | cutoff-docs |
| 123 | `47bf312282f7ddd9b1941074f497be1ee4304ea8` | 2026-07-29T22:47:30-06:00 | Suppress placeholder BMS faults during startup | cutoff-docs |
| 124 | `0afb0422cd56fee90237f6b8bcfb1d5ae72e8ae3` | 2026-07-29T22:53:52-06:00 | Harden Float Out Boy app data and LCM | cutoff-docs |
| 125 | `c6f80a24c531191d854de212017d7e70d6859c1c` | 2026-07-29T23:03:57-06:00 | Port Refloat cutoff package UI | cutoff-docs |
| 126 | `a14e732601e0c61c193de0b93b3509b24e24c7e8` | 2026-07-29T23:05:20-06:00 | Match cutoff firmware Mahony migration | cutoff-docs |
| 127 | `7e4ff40b78d67c071915035c49fc14be1c14d4e4` | 2026-07-29T23:08:58-06:00 | Test cutoff axis fault encoding | cutoff-docs |
| 128 | `245052ddffc0aa39d9c8d129c1134b94c3292ba9` | 2026-07-29T23:13:44-06:00 | Document Float Out Boy cutoff behavior | cutoff-docs |
| 129 | `a41bca7d8a6080f2c2a7705f62e068c4831ab75d` | 2026-07-30T14:00:42-06:00 | Merge current main into Refloat port | cutoff-docs |
| 130 | `9f9d6f9c623730fff5a26acbdabbe4897fa26af3` | 2026-07-30T12:59:55-06:00 | Preserve legacy ATR tune speeds | cutoff-docs |
| 131 | `b3262390b3d370facd534176a59bd6554cbe78d5` | 2026-07-30T13:06:23-06:00 | Port cutoff tune other fields | cutoff-docs |
| 132 | `c6a523edcf5b57bd0389c1485f360c072c0105ac` | 2026-07-30T13:08:53-06:00 | Port speed pushback tune field | cutoff-docs |
| 133 | `3e13eacf3f7f773a710ee614facca22277700b94` | 2026-07-30T14:01:20-06:00 | Remove obsolete package proof ledger | cutoff-docs |
| 134 | `b89a5f37038d8c63ddd0c70273b3b49481306e1e` | 2026-07-30T14:22:55-06:00 | Document Float Out Boy protocol and architecture | cutoff-docs |
| 135 | `41b023e952febf8d519e18d26645ec7c270af1b7` | 2026-07-30T14:33:35-06:00 | fixup! Document Float Out Boy protocol and architecture | cutoff-docs |
| 136 | `6dad2905a0a72fc2c26d963ebbec43e786f99ed8` | 2026-07-30T14:33:41-06:00 | Document cargo-vescpkg workflow and SDK capabilities | cutoff-docs |
| 137 | `8fddd2be4cb04cd53e4dc78b1d108a3c80a485e9` | 2026-07-30T14:35:53-06:00 | fixup! Document cargo-vescpkg workflow and SDK capabilities | cutoff-docs |
| 138 | `1c095c645e3d029ddde8a3102270ca8b53c3d44d` | 2026-08-02T13:11:28-06:00 | Document package UI asset authoring | cutoff-docs |
| 139 | `2ef68849f45f6686a909842a984c047496c53159` | 2026-08-02T13:19:23-06:00 | Compile official loopback example ports | cutoff-docs |
| 140 | `8d7e786e90e659d0d38a72981e6b4781b6f3450e` | 2026-08-02T13:23:35-06:00 | Fix custom data float assertion | cutoff-docs |
| 141 | `8dc1b4621fb7200710889b09f70ccd120b74288f` | 2026-08-02T13:30:12-06:00 | Document official example source mappings | cutoff-docs |
| 142 | `0d5fd5d23024aa9d0622e1d76495d507bfb60c39` | 2026-08-02T13:32:05-06:00 | Clarify Express host proof boundary | docs-arm |
| 143 | `57ea3669149ae898547d9d3b2bd31405e47ccc6c` | 2026-08-02T13:40:50-06:00 | Add restrained FOC audio smoke seam | docs-arm |
| 144 | `57a34c2045b885b48e875764b48e16739cafd556` | 2026-08-02T13:43:03-06:00 | Add typed audio beep probe | docs-arm |
| 145 | `b5f7eb38a9047e7848a7d9153e312f446d5a3fd9` | 2026-08-02T13:43:49-06:00 | Document restrained audio hardware workflow | docs-arm |
| 146 | `ffca5538216e4b2483cb51559945cf036b46dc01` | 2026-08-02T13:52:39-06:00 | Correct official extension source path | docs-arm |
| 147 | `4a5b9d054d9aad520ea263c2dae84cbb49cfdc26` | 2026-08-02T13:52:58-06:00 | Correct host protocol ownership documentation | docs-arm |
| 148 | `62a3adc3def16f0421947de8d615213ebf68514c` | 2026-08-02T13:56:18-06:00 | Document loopback thread example port | docs-arm |
| 149 | `12fa40a1f921763d9edc5c14aebd0a9c949b0450` | 2026-08-02T13:57:51-06:00 | Cover audio command in docs inventory test | docs-arm |
| 150 | `eddb5ea4aa0e2fa337b96bfc4d905ed14acc12af` | 2026-08-02T13:59:08-06:00 | Remove stale roadmap workspace inventory | docs-arm |
| 151 | `8b3b4cb3e0e2842ecef137fc5e678ab9979c2044` | 2026-08-02T13:59:45-06:00 | Refresh authoritative workspace inventory | docs-arm |
| 152 | `81c21c3fb9288a080a9443ab958f0bbb853f0bb8` | 2026-08-02T14:01:57-06:00 | Make alloc smoke ARM lint clean | docs-arm |
| 153 | `d680295439f55af124c2c2e13ccbd684a90286f0` | 2026-08-02T14:02:53-06:00 | Harden control loop example arithmetic | docs-arm |
| 154 | `0686243223fc65acf301320f7b8b693606401b45` | 2026-08-02T14:03:05-06:00 | Lint every example package for ARM | runtime-locks |
| 155 | `f9e5e2d380aa2b5cce693fa29432543d78d7f3e3` | 2026-08-04T13:52:13-06:00 | Inline Float Out Boy-specific support | runtime-locks |
| 156 | `7c79d807f9f600b7345bb704a6e7c8849ae280f3` | 2026-08-04T13:54:05-06:00 | Reuse realtime test tick helper | runtime-locks |
| 157 | `d585b451ed95cb43357f3c50618a6e87ae7b2358` | 2026-08-04T14:30:18-06:00 | Keep Float Out Boy startup within loader limits | runtime-locks |
| 158 | `f96b0161850023a8203347050adfba3aa733f6b9` | 2026-08-04T15:01:20-06:00 | Move sample-rate tracking into vescpkg | runtime-locks |
| 159 | `df3576bf1f3cda20bc0dddf5c7eac193fb3f5c43` | 2026-08-04T15:12:01-06:00 | Move recorder decimation into vescpkg | runtime-locks |
| 160 | `fb83fb174491c972a2293249fb5c4f03aaf3a056` | 2026-08-04T19:34:18-06:00 | Avoid LED reinitialization during tune swaps | runtime-locks |
| 161 | `6f1c39434eef8174e187adfa700605a82a22f7d5` | 2026-08-04T20:03:35-06:00 | Revert "Avoid LED reinitialization during tune swaps" | runtime-locks |
| 162 | `6748aeb861e5148b0d63f69a52ec2626f098a67e` | 2026-08-04T20:11:22-06:00 | Avoid LED resets without growing tune stack | runtime-locks |
| 163 | `7b8f98673fdb5b74038074e1ea48726e776e59d4` | 2026-08-04T20:21:35-06:00 | Revert "Avoid LED resets without growing tune stack" | runtime-locks |
| 164 | `f01d478a550c9655d707765f3003cd63dfad6a56` | 2026-08-04T20:30:05-06:00 | Apply runtime tunes with one reconfigure | runtime-locks |
| 165 | `b979c2ce7d63486192fc81b4bfa4d0ddb8f5698f` | 2026-08-05T14:34:43-06:00 | Release control state while preparing tunes | runtime-locks |
| 166 | `e5b4d13d510d562426849b0e18928681a2ab79f6` | 2026-08-05T14:55:47-06:00 | Release state while storing aux backup | runtime-locks |
| 167 | `30dea05b340e61bc3d14d800038539f293612fa9` | 2026-08-05T15:02:01-06:00 | Sample LED telemetry outside state lock | runtime-locks |
| 168 | `20203d0e44ce3fcf6290a03d1a23404712d2c403` | 2026-08-05T15:03:25-06:00 | Revert "Sample LED telemetry outside state lock" | runtime-locks |
| 169 | `e14501ef3281b418355f6fc3b9d3beba948e0117` | 2026-08-05T15:16:21-06:00 | Snapshot motor config outside shared state | snapshot-persistence |
| 170 | `7aa8dda34fe0d5af07b5fc979de7327afca5d2e2` | 2026-08-05T17:14:21-06:00 | Defer config persistence until safely stopped | snapshot-persistence |
| 171 | `61d9b01b34a816c52dcffe1b6764f52116104a94` | 2026-08-05T18:15:30-06:00 | Flatten FOB telemetry snapshots | snapshot-persistence |
| 172 | `060d5a1beea0c084128836821400135d541ff124` | 2026-08-05T18:52:42-06:00 | Reduce FOB LED and realtime plumbing | snapshot-persistence |
| 173 | `b977afacd6df387df3a6067cea3af7bab7be6df7` | 2026-08-05T19:13:12-06:00 | Parse FOB app data once | snapshot-persistence |
| 174 | `d30af50c9d543966ab18889436addda67017476f` | 2026-08-05T19:28:19-06:00 | Move smooth setpoint control into vescpkg-rs | shared-runtime |
| 175 | `021b874277691e6f563b244a17f76cf4e21c3f74` | 2026-08-05T19:48:21-06:00 | Move VESC recorder state into vescpkg-rs | shared-runtime |
| 176 | `9dd0b8dbcc48a1d40dcfa99bfdc4c8100c26f42a` | 2026-08-05T20:04:18-06:00 | Move deferred persistence into vescpkg-rs | shared-runtime |
| 177 | `321d7b7d82cd3f53e76ad8e219715039ff1a6262` | 2026-08-05T20:26:27-06:00 | Simplify typed remote and torque state | shared-runtime |
| 178 | `d7ce22ef461279a48a6c9717729e29f86c9d2383` | 2026-08-05T20:42:54-06:00 | Generalize VESC WS2812 peripheral support | shared-runtime |
| 179 | `d84a8be7e391bdfcc2f53e103bfbc3f637f4ea14` | 2026-08-05T21:02:38-06:00 | Move shared haptic timing into vescpkg-rs | shared-runtime |
| 180 | `1aa8c4104c525e13ef9f307ddc13e8f1b9887dad` | 2026-08-05T21:23:06-06:00 | Share typed package loop timing | shared-runtime |
| 181 | `36ee38965a4b687fdec12d085ba33a6d6957877b` | 2026-08-05T21:42:06-06:00 | Simplify typed ride modifier configuration | shared-runtime |
| 182 | `193cb59a02f17b4a63a8b0525e74526ce6b9b91c` | 2026-08-05T21:58:07-06:00 | Type directional current limits | shared-runtime |
| 183 | `12b5397c520ef064d85b0123baed0a827d419182` | 2026-08-05T22:09:01-06:00 | Share VESC data recorder response framing | shared-runtime |
| 184 | `2e15f5e12d460659c5ea7d75400f7ac1e3fea228` | 2026-08-05T22:16:52-06:00 | Move WS2812 DMA ownership into vescpkg-rs | shared-runtime |
| 185 | `de313b684032b632d46108b850c44edd02f9b8c0` | 2026-08-05T22:26:57-06:00 | Generate stack-safe FOB state defaults | final-typed-sdk |
| 186 | `a00e4e68e9bd4155f81658d266246b01fdb2d729` | 2026-08-05T22:53:04-06:00 | Simplify typed FOB runtime state | final-typed-sdk |
| 187 | `40677896a3dde0f7ce6b5257678f3fb3c161a215` | 2026-08-05T23:21:49-06:00 | Simplify typed FOB runtime configuration | final-typed-sdk |
| 188 | `66b3a750fdff18ad7f25425110add2f8efbf5da4` | 2026-08-05T23:39:27-06:00 | Simplify typed FOB control plumbing | final-typed-sdk |
| 189 | `2efd6e4e8fdfff040f6300659995c895663feec4` | 2026-08-06T00:08:16-06:00 | Simplify typed FOB protocol plumbing | final-typed-sdk |
| 190 | `6f84549fc6f4d4c066173eeb743a811db2904afe` | 2026-08-06T00:19:53-06:00 | Simplify typed LED transition state | final-typed-sdk |
| 191 | `68052f86f078b75868eae0dc4b8558f9f7e00b0d` | 2026-08-06T00:39:59-06:00 | Simplify typed FOB protocol timing | final-typed-sdk |
| 192 | `109bd67fc79d4f24e043df67c9c1dd1dc1ad223c` | 2026-08-06T01:04:58-06:00 | Move shared VESC BMS monitoring into vescpkg-rs | final-typed-sdk |
| 193 | `2f0f728722aff62d810b327b923d1bc13fa91d6a` | 2026-08-06T01:31:59-06:00 | Move shared VESC motor control into vescpkg-rs | final-typed-sdk |
| 194 | `eb4246860530d14dd8a1104b8df1ad3cbb4e8f1e` | 2026-08-06T09:22:20-06:00 | Document package-specific SDK policy | final-typed-sdk |

## Final proof

- `git diff --quiet HEAD eb424686 -- ':!docs/vescr-401-accounting.md'`
  succeeds on the corrected final branch; the manifest itself is the one
  intentional accounting artifact added after the immutable source tip.
- `git log --format=%G? origin/main..HEAD` reports 212 `G` statuses.
- The full gate passed on this exact final tree: 1,267 workspace tests, 405
  SDK tests, 416 alloc/math SDK tests, doctests, host/ARM checks, package
  builds, and docs.
- `nix develop -c ./tools/fob-production-tokei.sh` reports 69 Rust files,
  13,036 Rust lines, and 9,942 production Rust LOC.
