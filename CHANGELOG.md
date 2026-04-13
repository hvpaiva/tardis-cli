
# Changelog
All notable changes to **TARDIS** will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

- - -
## [v0.2.0](https://github.com/hvpaiva/tardis-cli/compare/b88e73c304d8681ce1fbd034a633fa1a1256b83c..v0.2.0) - 2026-04-13
#### Features
- (**cli**) add TARDIS_NOW env var as alternative to --now flag - ([4ec1f2d](https://github.com/hvpaiva/tardis-cli/commit/4ec1f2d9ab45f150915c7e177453ffcded88e253)) - Highlander Paiva
- (**cli**) propagate --verbose flag to all subcommand handlers - ([1e643c5](https://github.com/hvpaiva/tardis-cli/commit/1e643c58106fb589bfc867b023b74a27b506246d)) - Highlander Paiva
- (**cli**) add DiffOutput enum, emit_json helper, and diff output routing - ([6fcee8e](https://github.com/hvpaiva/tardis-cli/commit/6fcee8e88bd1525edb2a6cc94391897508a6c0d8)) - Highlander Paiva
- (**cli**) extract CLI definitions into shared cli_defs.rs module - ([4be18ca](https://github.com/hvpaiva/tardis-cli/commit/4be18ca16d5bca35a3adb3165fa9b58fadadf0c5)) - Highlander Paiva
- (**cli**) add configurable delimiter flag to td range - ([20d0810](https://github.com/hvpaiva/tardis-cli/commit/20d08100943ec68dbafbd2305d0509b63c4ad0ea)) - Highlander Paiva
- (**cli**) add range subcommand - ([dc3c7d5](https://github.com/hvpaiva/tardis-cli/commit/dc3c7d5f4489e128e25ce474520c3718a0041fb3)) - Highlander Paiva
- (**cli**) wire verbose diagnostics in main.rs - ([26c450c](https://github.com/hvpaiva/tardis-cli/commit/26c450cdfc9e3cacd239c07e8e3c9988d8b1908c)) - Highlander Paiva
- (**cli**) add --verbose flag to CLI and Command, fix errors.rs doc - ([c855edf](https://github.com/hvpaiva/tardis-cli/commit/c855edfffa4ba2ca19a5656fc212b4b91dfac060)) - Highlander Paiva
- (**cli**) implement handle_tz() and handle_info() subcommand handlers - ([cb7e9fb](https://github.com/hvpaiva/tardis-cli/commit/cb7e9fbee0d0aa995061c028b1a9850e6c7ac88e)) - Highlander Paiva
- (**cli**) implement diff/convert handlers, range dispatch, and batch refactor - ([0689a77](https://github.com/hvpaiva/tardis-cli/commit/0689a77d16578c18c5ab9fcb440674fe167b3dad)) - Highlander Paiva
- (**cli**) add CLI subcommand definitions and batch mode helpers - ([4b9737f](https://github.com/hvpaiva/tardis-cli/commit/4b9737fd0fa0ec1dd1b7b7d3186ab9d0a95705e4)) - Highlander Paiva
- (**cli**) add subcommands, JSON output, epoch support, batch mode, no-newline - ([daf8c03](https://github.com/hvpaiva/tardis-cli/commit/daf8c03791a024bf73c1d13f4137b057cc1fa952)) - Highlander Paiva
- (**config**) replace config crate with direct toml parsing - ([013c4ab](https://github.com/hvpaiva/tardis-cli/commit/013c4ab45b59fb20b0341ddc4b26ed84c4eefdc3)) - Highlander Paiva
- (**core**) activate ereyesterday keyword and remove dead parse_range API - ([25834f5](https://github.com/hvpaiva/tardis-cli/commit/25834f5be649b6c9109f748ec984f037aaa47d68)) - Highlander Paiva
- (**core**) add AM/PM time expressions and "at same time" support - ([dc9c0c2](https://github.com/hvpaiva/tardis-cli/commit/dc9c0c2f689597770efd6e2d37ead56711da95a0)) - Highlander Paiva
- (**core**) multi-line typo suggestion format with yellow color - ([8c1de41](https://github.com/hvpaiva/tardis-cli/commit/8c1de41292270c3ff873211dcf33ba41b2474926)) - Highlander Paiva
- (**core**) add range resolution and granularity parsing - ([26a449c](https://github.com/hvpaiva/tardis-cli/commit/26a449c14ac4e4b827e074a07983ba2dbf38022e)) - Highlander Paiva
- (**core**) implement resolve_boundary for all 32 BoundaryKind variants and HourOnly time - ([5013b30](https://github.com/hvpaiva/tardis-cli/commit/5013b3046383c092475245394b51982d3919d748)) - Highlander Paiva
- (**core**) add grammar productions for operator-prefixed offsets, boundary keywords, compound durations, and Nh time suffix - ([d86ade4](https://github.com/hvpaiva/tardis-cli/commit/d86ade48c0cda31a73aaf83db9d91c5d2ed62591)) - Highlander Paiva
- (**core**) add 32 boundary keywords to EN/PT locales and fix lexer sign-position - ([cd0066a](https://github.com/hvpaiva/tardis-cli/commit/cd0066a7364f14553dd1e868e10367ca06fe0a4d)) - Highlander Paiva
- (**core**) add BoundaryKind enum, Token::Boundary, DateExpr::Boundary, TimeExpr::HourOnly - ([9fffa01](https://github.com/hvpaiva/tardis-cli/commit/9fffa01cc94893a2bd23dffab76847c785b8e9c6)) - Highlander Paiva
- (**core**) add abbreviated duration unit keywords to PT locale - ([550fb63](https://github.com/hvpaiva/tardis-cli/commit/550fb634f4be0813e98aa5abc76f978d5f4b43e7)) - Highlander Paiva
- (**core**) add abbreviated duration unit keywords to EN locale - ([889a09f](https://github.com/hvpaiva/tardis-cli/commit/889a09fda66ac20e3a84fac46662c00bbd1ba56d)) - Highlander Paiva
- (**core**) add verbose flag and public API annotations - ([0230fff](https://github.com/hvpaiva/tardis-cli/commit/0230fff952ae0f4a3fd8b1f5314cc786800d6746)) - Highlander Paiva
- (**core**) expand benchmark suite from 1 to 31 functions - ([7e0ac0b](https://github.com/hvpaiva/tardis-cli/commit/7e0ac0b4c72fd52a3e9d86180c7917f81bf09e63)) - Highlander Paiva
- (**core**) implement Portuguese locale, Ereyesterday AST/resolver, and prefix-ago grammar - ([d54e749](https://github.com/hvpaiva/tardis-cli/commit/d54e749f305953f2132c8cb40075f68d6ecf66fb)) - Highlander Paiva
- (**core**) wire locale through CLI/Config/App/parser pipeline and refactor lexer - ([f9120a7](https://github.com/hvpaiva/tardis-cli/commit/f9120a7ff969c21adf1417517b03abc1cad82c76)) - Highlander Paiva
- (**core**) create locale module with Locale trait, EN locale, registry, and detection - ([a4d0ae7](https://github.com/hvpaiva/tardis-cli/commit/a4d0ae73b022e11b033d7e562207476809cf4148)) - Highlander Paiva
- (**core**) big-bang chrono-to-jiff migration across all source files - ([16787b6](https://github.com/hvpaiva/tardis-cli/commit/16787b604646e8c0b480e613575e369ef783eb82)) - Highlander Paiva
- (**core**) create golden snapshot test suite with insta - ([dfd4125](https://github.com/hvpaiva/tardis-cli/commit/dfd41257b11e7acc3ea701c00d8e108d092c418e)) - Highlander Paiva
- (**infra**) eliminate build.rs, move man page generation to runtime - ([852dee8](https://github.com/hvpaiva/tardis-cli/commit/852dee803c3dbc8e799f1dc9564ee66d59eea435)) - Highlander Paiva
- (**parser**) simplify parser API and remove locale parameter - ([7b4cc4d](https://github.com/hvpaiva/tardis-cli/commit/7b4cc4d56018d2c00035e9a17c282e269c25fd62)) - Highlander Paiva
- (**parser**) inline EN keywords into lexer - ([f7efda9](https://github.com/hvpaiva/tardis-cli/commit/f7efda9663e16b33dede22ba968b140e78144ba3)) - Highlander Paiva
- (**parser**) reclassify last-week/month/year as Offset - ([69973ac](https://github.com/hvpaiva/tardis-cli/commit/69973ac32cdc717acac81b962ad012fb4dc31b78)) - Highlander Paiva
- (**parser**) implement resolver for arithmetic and range expressions - ([c467fe3](https://github.com/hvpaiva/tardis-cli/commit/c467fe31043fee7649c425f0fce246095d0361cd)) - Highlander Paiva
- (**parser**) extend lexer and grammar for arithmetic and range expressions - ([aad78fb](https://github.com/hvpaiva/tardis-cli/commit/aad78fba42291e590f5e9e4d31a260f5744b9d01)) - Highlander Paiva
- (**parser**) update golden tests, benchmarks, and integration tests for custom parser - ([e129b9b](https://github.com/hvpaiva/tardis-cli/commit/e129b9b94613fff088440752fda9edab7c8ea316)) - Highlander Paiva
- (**parser**) swap human-date-parser with custom parser in core.rs - ([9b1b63b](https://github.com/hvpaiva/tardis-cli/commit/9b1b63b48dfd0309c443f34842c1d2d272fcd89e)) - Highlander Paiva
- (**parser**) implement AST resolver and wire full parse pipeline - ([3e24d9f](https://github.com/hvpaiva/tardis-cli/commit/3e24d9f864ff4ddb079354f1635fe3856f589af7)) - Highlander Paiva
- (**parser**) implement recursive descent grammar parser - ([5a72634](https://github.com/hvpaiva/tardis-cli/commit/5a726349bbb7860823df713d4f7f5b8d4adbf6b7)) - Highlander Paiva
- (**parser**) implement character-by-character lexer with keyword recognition - ([bc42579](https://github.com/hvpaiva/tardis-cli/commit/bc42579c8aa169d55b921b6fc2626e524451a470)) - Highlander Paiva
- (**parser**) define parser type foundations and module scaffold - ([31e3673](https://github.com/hvpaiva/tardis-cli/commit/31e36732491ef402e7a0d6eec30d685f32b1863a)) - Highlander Paiva
- (**parser**) expand benchmark suite with baselines for all expression categories - ([7b6f9c2](https://github.com/hvpaiva/tardis-cli/commit/7b6f9c2811a39c7bb5bec49b5cab401038a4be6d)) - Highlander Paiva
#### Bug Fixes
- (**cli**) clean up man page quality issues - ([9934163](https://github.com/hvpaiva/tardis-cli/commit/9934163e0f94b4f897095ae6afc6b2504e6e31ab)) - Highlander Paiva
- (**cli**) correct TARDIS acronym to Time And Relative Date Input Simplifier - ([5c632a2](https://github.com/hvpaiva/tardis-cli/commit/5c632a2f03671fe58c12e187936cc054d3e8f27f)) - Highlander Paiva
- (**cli**) apply two-tier help pattern to --skip-errors flag - ([7839488](https://github.com/hvpaiva/tardis-cli/commit/7839488a42cb29c0cc76bedcc142712766af764f)) - Highlander Paiva
- (**cli**) add FORMAT-SPECIFIERS.md to trycmd harness and fix preset example - ([e46bc13](https://github.com/hvpaiva/tardis-cli/commit/e46bc1306276f1a33d15612616c2f53637f3c47d)) - Highlander Paiva
- (**cli**) correct help text for epoch escaping, periods and boundaries - ([89d826b](https://github.com/hvpaiva/tardis-cli/commit/89d826b7081bc26f857305104b04960f8685035e)) - Highlander Paiva
- (**cli**) fix tz offset format and skip-errors terminal UX - ([ecd6061](https://github.com/hvpaiva/tardis-cli/commit/ecd6061fe3922a5e473ad1cc3c8340045c40b9bf)) - Highlander Paiva
- (**core**) make suggestion tests terminal-agnostic - ([066393c](https://github.com/hvpaiva/tardis-cli/commit/066393cd73e84a6d37bf19c606576f6023701251)) - Highlander Paiva
- (**core**) accept bare epoch timestamps in td convert without @ prefix - ([5960fd0](https://github.com/hvpaiva/tardis-cli/commit/5960fd039d4d9b9855ff96b1009caaf1864c3efb)) - Highlander Paiva
- (**core**) emit Plus/Dash operators in no-space arithmetic expressions - ([24a1dcf](https://github.com/hvpaiva/tardis-cli/commit/24a1dcf9a71971d0c94ac19d7739d340d1759a15)) - Highlander Paiva
- (**core**) add RFC 3339/ISO 8601 fallback in convert and parse - ([22759cd](https://github.com/hvpaiva/tardis-cli/commit/22759cd1e0ec2a73d6e2b82071dab5e5ea6b285c)) - Highlander Paiva
- (**core**) add input echo in parse errors and propagate EN fallback suggestions - ([1c81633](https://github.com/hvpaiva/tardis-cli/commit/1c816332f506df824545466077a440781847326d)) - Highlander Paiva
- (**core**) add EN fallback for non-EN locale parse failures - ([ded9791](https://github.com/hvpaiva/tardis-cli/commit/ded97917a9a3b10fd68e7dc3f8b2fb1a571d2b5a)) - Highlander Paiva
- (**core**) fix benchmark expressions for custom parser compatibility - ([0311654](https://github.com/hvpaiva/tardis-cli/commit/0311654b5953694fcc375ebb104a6100ec51f87f)) - Highlander Paiva
- (**core**) restore source files after locale integration - ([e865528](https://github.com/hvpaiva/tardis-cli/commit/e86552862791bf009473121e238727513adb6e72)) - Highlander Paiva
- (**core**) remove ghost deps and add deny lints to main.rs - ([0007061](https://github.com/hvpaiva/tardis-cli/commit/00070616b21befdedfff59ec7063bee6c0c27480)) - Highlander Paiva
- (**core**) update integration test for jiff error message format - ([e14ba75](https://github.com/hvpaiva/tardis-cli/commit/e14ba75d51f6326c246a8039c7179798a1d230e7)) - Highlander Paiva
- (**core**) remove unused import in golden test suite - ([b227b9f](https://github.com/hvpaiva/tardis-cli/commit/b227b9f30a01358903d521000905775badbadc10)) - Highlander Paiva
- (**core**) resolve ambiguous errors, dead deps, stdin bug, and code reorganization - ([2bb472c](https://github.com/hvpaiva/tardis-cli/commit/2bb472c448f5ecef3130884e4adad31c4e6edbeb)) - Highlander Paiva
- (**infra**) use cargo update --workspace in pre-bump hooks - ([a1c65e8](https://github.com/hvpaiva/tardis-cli/commit/a1c65e8257b36c5d094e64648b9ed0be27e867c1)) - Highlander Paiva
- (**infra**) add cocogitto separator to CHANGELOG.md - ([0bb2216](https://github.com/hvpaiva/tardis-cli/commit/0bb221602fb936f2c7f541e8f82420d1a7148c31)) - Highlander Paiva
- (**infra**) tolerate exit code 1 in skip-errors smoke test - ([3313282](https://github.com/hvpaiva/tardis-cli/commit/33132824ad5f235fd640a21a6805c8f0b5d3ee84)) - Highlander Paiva
- (**infra**) pin pandoc 3.6.4 in man page CI job - ([f6a556e](https://github.com/hvpaiva/tardis-cli/commit/f6a556ea9675ccca4048dd35a8d1d1cf310e9e71)) - Highlander Paiva
- (**infra**) tolerate pandoc version differences in man page CI check - ([a627390](https://github.com/hvpaiva/tardis-cli/commit/a6273902d0af3796655cf9cd4676d48475605992)) - Highlander Paiva
- (**infra**) use pre-built semver-checks binary and fix diff smoke test - ([532a657](https://github.com/hvpaiva/tardis-cli/commit/532a657563bc2bddb0ec90a3129dfbaa8cf95afd)) - Highlander Paiva
- (**infra**) fix broken smoke test using unsupported range syntax - ([89ce640](https://github.com/hvpaiva/tardis-cli/commit/89ce640c1ea6f39e8c8de8d09d41336eda5a7f24)) - Highlander Paiva
- (**infra**) remove stale locale references from smoke tests and docs - ([993b2a0](https://github.com/hvpaiva/tardis-cli/commit/993b2a085d9b3dc2914c18cb5ab27ad2c70bd384)) - Highlander Paiva
- (**infra**) correct cargo-flamegraph crate name in dev-setup.sh - ([1ecc4c5](https://github.com/hvpaiva/tardis-cli/commit/1ecc4c5e0e355b85acf7a21611869df306b0d2ed)) - Highlander Paiva
- (**infra**) add cargo-vet publisher trust and fix CHANGELOG cog separator - ([5dc96d6](https://github.com/hvpaiva/tardis-cli/commit/5dc96d68723815c4763ab9d8fc2b39c7f7761028)) - Highlander Paiva
- (**infra**) remove unused license entries and fix Cargo.toml metadata - ([1e50be9](https://github.com/hvpaiva/tardis-cli/commit/1e50be90706380e3afc573c25cd13caf3c05f3ec)) - Highlander Paiva
- (**infra**) remove inline comment from .gitattributes - ([138db93](https://github.com/hvpaiva/tardis-cli/commit/138db93c027c4e344e2e4a5a5c6bff11ef8504df)) - Highlander Paiva
- restore insta dependency lost in merge conflict resolution - ([f4d1b63](https://github.com/hvpaiva/tardis-cli/commit/f4d1b6316e744f412bedec50163937270c444858)) - Highlander Paiva
#### Documentation
- (**cli**) restore trivia section in README - ([911377b](https://github.com/hvpaiva/tardis-cli/commit/911377bfefa5d821a1ed5221e6cfef4d074aa895)) - Highlander Paiva
- (**cli**) remove --now from all doc examples using harness env vars - ([9c0e70b](https://github.com/hvpaiva/tardis-cli/commit/9c0e70b3b001a8bff56b97555559d99fdf255c2a)) - Highlander Paiva
- (**cli**) remove --now pollution from doc examples using TARDIS_NOW - ([e1edf4b](https://github.com/hvpaiva/tardis-cli/commit/e1edf4b4135054df5ddb87649d158bd18505c4f3)) - Highlander Paiva
- (**cli**) restore TARDIS logo in README - ([cc40c39](https://github.com/hvpaiva/tardis-cli/commit/cc40c3914d7b3f63d3907c11a2ee72f3f2f32938)) - Highlander Paiva
- (**cli**) remove comparison table from README - ([01dc9d9](https://github.com/hvpaiva/tardis-cli/commit/01dc9d98cd7f0d3bfe4bbbf6791832d4c3f8ada8)) - Highlander Paiva
- (**cli**) restore "3pm" in README now that AM/PM is supported - ([32ce7f1](https://github.com/hvpaiva/tardis-cli/commit/32ce7f17474c85ada7f9cbc790b7f95cb8cf3c5d)) - Highlander Paiva
- (**cli**) add AM/PM and same-time expression docs and integration tests - ([8989483](https://github.com/hvpaiva/tardis-cli/commit/898948392bdc6b23e1ea1b073fa25099eb89cd67)) - Highlander Paiva
- (**cli**) slim README to portal, add trycmd test harness and man recipe - ([75bf418](https://github.com/hvpaiva/tardis-cli/commit/75bf418a4696a5e7542d8474ce8e3efd1bca44d8)) - Highlander Paiva
- (**cli**) add man page summary and subcommands reference - ([df1d2ae](https://github.com/hvpaiva/tardis-cli/commit/df1d2aeaa9bb7068a0fb2fef3790620e5d0879ca)) - Highlander Paiva
- (**cli**) add range, config, completions man pages and subcommands reference - ([d85bf9e](https://github.com/hvpaiva/tardis-cli/commit/d85bf9e8066bdc45662208996e9b46d4d0a8e037)) - Highlander Paiva
- (**cli**) add main td(1) and subcommand man pages for diff, convert, tz, info - ([3888201](https://github.com/hvpaiva/tardis-cli/commit/3888201019d22d4f490aed37edf520361dccabf9)) - Highlander Paiva
- (**cli**) create format specifiers and configuration reference - ([93b078f](https://github.com/hvpaiva/tardis-cli/commit/93b078fa17624a650d0a8e34aa592c903e4c4bfb)) - Highlander Paiva
- (**cli**) create comprehensive expression reference - ([af80c46](https://github.com/hvpaiva/tardis-cli/commit/af80c46030b27c8144391a24306ec4d80e4dab5a)) - Highlander Paiva
- (**infra**) generate man page snapshots, add CI validation job - ([958a9d2](https://github.com/hvpaiva/tardis-cli/commit/958a9d2a09063ce68abb832296610c9656a9996a)) - Highlander Paiva
- (**infra**) rewrite README as full showcase document - ([f7cdd72](https://github.com/hvpaiva/tardis-cli/commit/f7cdd725918242f34d8206b70e618ee7e689bc8d)) - Highlander Paiva
- (**infra**) add cargo-vet mention to SECURITY.md - ([4b0a839](https://github.com/hvpaiva/tardis-cli/commit/4b0a839b7eebf2fe34c181ee6e2787be71b20751)) - Highlander Paiva
- (**infra**) update CONTRIBUTING.md with parser/locale modules and new features - ([9f09944](https://github.com/hvpaiva/tardis-cli/commit/9f099444813bff8daf30782fc0abd59129303479)) - Highlander Paiva
- update README with all new features and badges - ([758129b](https://github.com/hvpaiva/tardis-cli/commit/758129b8fb20bde27cf1e9ad81eee592da5a1a40)) - Highlander Paiva
- add governance docs and update contributing guide - ([7dda3b2](https://github.com/hvpaiva/tardis-cli/commit/7dda3b279cbd4fe092cfa16b8340de4dc06d32cd)) - Highlander Paiva
- resize logo image - ([6f451ab](https://github.com/hvpaiva/tardis-cli/commit/6f451ab5800d1e6165715adb7507d0c4fcdaf009)) - hvpaiva
- improve documentation - ([0ed257b](https://github.com/hvpaiva/tardis-cli/commit/0ed257b88a41b30a48ea1456dfa8d31a2b8972d9)) - hvpaiva
- added CHANGELOG - ([64a29eb](https://github.com/hvpaiva/tardis-cli/commit/64a29ebf13f0ccddbdefc36e812e0add29f9a4fb)) - hvpaiva
#### Tests
- (**cli**) update docs and integration tests for time validation rules - ([a6b909d](https://github.com/hvpaiva/tardis-cli/commit/a6b909d5e5ecc8abdf13d7abc1eb3f247149f685)) - Highlander Paiva
- (**cli**) add README and man page validation to trycmd harness - ([7f6d4fc](https://github.com/hvpaiva/tardis-cli/commit/7f6d4fc473eea702b721f2846513c11309b4ca2c)) - Highlander Paiva
- (**cli**) add failing tests for diff --output modes and JSON piped compactness - ([6811ce4](https://github.com/hvpaiva/tardis-cli/commit/6811ce4dc1eab9e1062db0d40ee0d25a0b2f055c)) - Highlander Paiva
- (**cli**) add tests for range delimiter flag - ([4242153](https://github.com/hvpaiva/tardis-cli/commit/424215358156ad896fbd2fde0ffa42bc8d5563bd)) - Highlander Paiva
- (**cli**) comprehensive integration tests for all subcommands and range output - ([dd26cda](https://github.com/hvpaiva/tardis-cli/commit/dd26cda0000e0972a409cee234ba97e4496305df)) - Highlander Paiva
- (**core**) expand golden tests with boundaries, ranges, subcommands - ([424f684](https://github.com/hvpaiva/tardis-cli/commit/424f6843ad232214aa5ab77eda0eb87042ed1adc)) - Highlander Paiva
- (**core**) add CLI integration tests for missing boundary keywords - ([6f2e240](https://github.com/hvpaiva/tardis-cli/commit/6f2e2400e77ce5968e3d318a58828df6e35aeab7)) - Highlander Paiva
- (**core**) add failing test for ereyesterday and migrate parse_range callers - ([3bcefba](https://github.com/hvpaiva/tardis-cli/commit/3bcefba1032bcc3e8f0390e12ef89e856159a62a)) - Highlander Paiva
- (**core**) add exhaustive integration tests for expression coverage - ([3359ef4](https://github.com/hvpaiva/tardis-cli/commit/3359ef4c955b0bf704cca1298c997a0ed5be49b2)) - Highlander Paiva
- (**core**) update integration tests for last-week single-date behavior - ([5f03eeb](https://github.com/hvpaiva/tardis-cli/commit/5f03eebbf27444b0a2177561cea0817181ff9dc9)) - Highlander Paiva
- (**core**) add integration tests for EN fallback suggestions and input echo - ([2f8db92](https://github.com/hvpaiva/tardis-cli/commit/2f8db925c68205e51a7737c40a619b39df2be462)) - Highlander Paiva
- (**core**) comprehensive locale integration tests for EN and PT end-to-end parsing - ([6720f23](https://github.com/hvpaiva/tardis-cli/commit/6720f23e3eb236cd3e4d0c10218a56a444ae9871)) - Highlander Paiva
- (**infra**) add smoke tests for boundaries, epoch, no-newline, completions - ([51b771d](https://github.com/hvpaiva/tardis-cli/commit/51b771d17086cd388ff45145f6a403c6bd9e27d8)) - Highlander Paiva
- (**infra**) expand smoke tests from 3 to 12 - ([01d6035](https://github.com/hvpaiva/tardis-cli/commit/01d60352a99926238037908a8f3c7f71a56db4f2)) - Highlander Paiva
- (**infra**) add man page generation integration test - ([48d062f](https://github.com/hvpaiva/tardis-cli/commit/48d062f5117b7a8f3c374174f098d8bc07b08269)) - Highlander Paiva
- comprehensive integration tests for all new features and error paths - ([e1cd37a](https://github.com/hvpaiva/tardis-cli/commit/e1cd37a25f827f03d3b257c26b4d5d9ea35d08ad)) - Highlander Paiva
#### Build system
- (**deps**) update transitive dependencies via cargo update - ([7b53d6a](https://github.com/hvpaiva/tardis-cli/commit/7b53d6acb8034bd17ae4ff448a3413288479a867)) - Highlander Paiva
- (**deps**) clean up unused license allowances in deny.toml - ([4f19321](https://github.com/hvpaiva/tardis-cli/commit/4f19321f09cf1e36e2d29872ee392d80d9276eaf)) - Highlander Paiva
- (**deps**) bump all dependencies to latest compatible versions - ([8c48071](https://github.com/hvpaiva/tardis-cli/commit/8c480713ff59945c8e8fe2006d46301339218bb1)) - Highlander Paiva
- (**infra**) replace fake audits with publisher trusts in cargo-vet - ([7009485](https://github.com/hvpaiva/tardis-cli/commit/7009485d52997e1422886e37c5b63f1e955701e9)) - Highlander Paiva
- (**infra**) adopt cocogitto for conventional commits and release automation - ([ce95c0b](https://github.com/hvpaiva/tardis-cli/commit/ce95c0b8add17e4546c81432cf29a14671f29eeb)) - Highlander Paiva
- (**infra**) add quality tooling configs - ([b81d0fa](https://github.com/hvpaiva/tardis-cli/commit/b81d0fac0d414153abd948ecc1942b7276f9ca9a)) - Highlander Paiva
- fixed release CI - ([8b1618f](https://github.com/hvpaiva/tardis-cli/commit/8b1618fc599b34e296cc93686ddf3c6a99330ab1)) - hvpaiva
- updated vet audits email - ([32b694c](https://github.com/hvpaiva/tardis-cli/commit/32b694cadb26fc8e32264c82da34c09e097b90cd)) - hvpaiva
- improved cli documentation - ([ede96a5](https://github.com/hvpaiva/tardis-cli/commit/ede96a57a8ffa2034fe95d9000793dfdcd154cf9)) - hvpaiva
- improve contributing - ([f551652](https://github.com/hvpaiva/tardis-cli/commit/f551652e22da1dacb58bb6d94f3d1047db94c86c)) - hvpaiva
- fixed smoke tests portability - ([4a40b38](https://github.com/hvpaiva/tardis-cli/commit/4a40b385be5c1a8159795e41faf8770e5674613e)) - hvpaiva
- added include config in cargo - ([8c8423d](https://github.com/hvpaiva/tardis-cli/commit/8c8423dcac8b12bf06e5c4b1a973c987d069ee25)) - hvpaiva
- fixed github actions with wrong flag in cargo vet and refactor release flow - ([9b75679](https://github.com/hvpaiva/tardis-cli/commit/9b7567928a159e20730da4009faaeb46005b9ff8)) - hvpaiva
- fixed github actions with rust imports - ([b88e73c](https://github.com/hvpaiva/tardis-cli/commit/b88e73c304d8681ce1fbd034a633fa1a1256b83c)) - hvpaiva
#### Continuous Integration
- (**infra**) add cargo-semver-checks job for API compatibility - ([f495025](https://github.com/hvpaiva/tardis-cli/commit/f4950252ff0afdb26627885324ccb26463b211f7)) - Highlander Paiva
- (**infra**) add CycloneDX SBOM generation to release workflow - ([ed62d45](https://github.com/hvpaiva/tardis-cli/commit/ed62d45b4c21e4f92d80324483397e459946a7ee)) - Highlander Paiva
- (**infra**) rewrite monolithic check job into parallel CI jobs - ([3e01523](https://github.com/hvpaiva/tardis-cli/commit/3e015231f5dd91e58be89b4a001b33142211610b)) - Highlander Paiva
- rewrite CI/CD workflows and add GitHub templates - ([97baaaf](https://github.com/hvpaiva/tardis-cli/commit/97baaaf5c76aa6aa29d5d0ceaf4e8a87924a0928)) - Highlander Paiva
- fixed pipeline for smoke tests - ([9a8bcca](https://github.com/hvpaiva/tardis-cli/commit/9a8bccaa8380c439f62a7a5050f863bf56b7c04a)) - hvpaiva
#### Refactoring
- (**cli**) clean up test names and remove dead duplicates - ([34b73fc](https://github.com/hvpaiva/tardis-cli/commit/34b73fc72f88ac981fe683913fb396d31554ca70)) - Highlander Paiva
- (**core**) add #[must_use] to public types and functions - ([a4fe65b](https://github.com/hvpaiva/tardis-cli/commit/a4fe65b75de5476529724452d578f8233d85b0a4)) - Highlander Paiva
- (**core**) remove external tool references and rename boundary tests - ([1b53e0d](https://github.com/hvpaiva/tardis-cli/commit/1b53e0dc1caea083d5ec68a3e1a3b2802c4dee94)) - Highlander Paiva
- (**core**) remove non-doc comments and add missing documentation - ([d403abc](https://github.com/hvpaiva/tardis-cli/commit/d403abca387d5c052c193a3ec9b612029ddeba26)) - Highlander Paiva
- (**core**) separate error formatting from terminal colorization - ([97d2f9b](https://github.com/hvpaiva/tardis-cli/commit/97d2f9b87fc519eea208113766b52c92219f61a8)) - Highlander Paiva
- (**core**) add #[non_exhaustive] to all public parser enums - ([cf10499](https://github.com/hvpaiva/tardis-cli/commit/cf10499377bfa5ca588d19082ac1b54e5a54a723)) - Highlander Paiva
- (**core**) unify time notation validation and remove standalone time - ([43848ad](https://github.com/hvpaiva/tardis-cli/commit/43848ad8e837e2fe2ca9c5442f4fe400a3a112d8)) - Highlander Paiva
- (**core**) delete locale module and locale tests - ([3e31829](https://github.com/hvpaiva/tardis-cli/commit/3e31829551110f5b0ebc3425ca6c9d169749f969)) - Highlander Paiva
- (**core**) remove locale module export from lib.rs - ([b2419eb](https://github.com/hvpaiva/tardis-cli/commit/b2419ebc818407e05dc33842a8a1f4af445d0fb6)) - Highlander Paiva
- (**infra**) remove locale references from README.md - ([ec8f993](https://github.com/hvpaiva/tardis-cli/commit/ec8f9934015942d8ca95e027ac15a5b05a89a1cd)) - Highlander Paiva
- (**parser**) remove file-level dead_code allows from parser modules - ([0a6c3dd](https://github.com/hvpaiva/tardis-cli/commit/0a6c3ddb5253bf9b7cc52a355c69421ed22f59c1)) - Highlander Paiva
- simplify build.rs with minimal CLI mirror and shell completion generation - ([b879f42](https://github.com/hvpaiva/tardis-cli/commit/b879f4263f79df18388198bcc7a99cc7c7f5758c)) - Highlander Paiva
#### Miscellaneous Chores
- (**core**) remove dead dependencies and add lint guards - ([0bc1be7](https://github.com/hvpaiva/tardis-cli/commit/0bc1be7ed9c4497c136270d82efa212d4924080e)) - Highlander Paiva
- (**infra**) restore exclude list and remove stale locale scope - ([8fd8881](https://github.com/hvpaiva/tardis-cli/commit/8fd8881f8d520d3b6db14a5db0bff2f46bd9d804)) - Highlander Paiva
- (**infra**) clean up packaging and config before release - ([864ec2a](https://github.com/hvpaiva/tardis-cli/commit/864ec2a30030f523a7034ef86ccbf03a0c50c8e4)) - Highlander Paiva
- (**infra**) remove CLAUDE.md from version control - ([621f501](https://github.com/hvpaiva/tardis-cli/commit/621f501966d41b90e84a76a179ddd924d0ad50d7)) - Highlander Paiva
- (**infra**) ignore generated artifacts and tool config - ([9d5da95](https://github.com/hvpaiva/tardis-cli/commit/9d5da95f150e9e4c1efab4225b7b548dc5558b26)) - Highlander Paiva
- (**infra**) remove accidentally tracked internal artifact - ([468faca](https://github.com/hvpaiva/tardis-cli/commit/468faca079de149a1c9de93fd4de22b68b5c045a)) - Highlander Paiva
- (**infra**) exclude internal docs from tracking and harden .gitignore - ([2e11215](https://github.com/hvpaiva/tardis-cli/commit/2e11215f38978e604ffb1fb8c5a68816b199e08d)) - Highlander Paiva
- (**infra**) add cargo-semver-checks and cargo-sbom to dev-setup.sh - ([b467a4f](https://github.com/hvpaiva/tardis-cli/commit/b467a4f8ed0404b346895815b40d3f45ab7b6e4e)) - Highlander Paiva
- (**infra**) add vet, sbom, and semver-check justfile recipes - ([4423d37](https://github.com/hvpaiva/tardis-cli/commit/4423d3717ff52ee9de03aa2227969e9a44f2eb3f)) - Highlander Paiva
- (**infra**) add open-pull-requests-limit to dependabot config - ([888d975](https://github.com/hvpaiva/tardis-cli/commit/888d97569d08add326813045d35b1da359936c7d)) - Highlander Paiva
- (**infra**) add parser/locale scopes and ignore merge commits in cog.toml - ([938fd41](https://github.com/hvpaiva/tardis-cli/commit/938fd419b48818776863b66b004d0cbdf082ef9c)) - Highlander Paiva
- merge feature branch - ([75e8380](https://github.com/hvpaiva/tardis-cli/commit/75e8380e81df1ce16644ce917b420c08ca502afa)) - Highlander Paiva
- merge feature branch - ([c804634](https://github.com/hvpaiva/tardis-cli/commit/c804634623e7fd71c8d9c4f76ee948915ae1904b)) - Highlander Paiva
- merge feature branch - ([ddc9a9d](https://github.com/hvpaiva/tardis-cli/commit/ddc9a9de4bfc77206865cf6edd835143bc4748b6)) - Highlander Paiva
- merge feature branch - ([0898266](https://github.com/hvpaiva/tardis-cli/commit/08982664f163237486d74bb3af502c11f684bef5)) - Highlander Paiva
- merge feature branch - ([2702505](https://github.com/hvpaiva/tardis-cli/commit/2702505a06a83bbbaa081effdeebf43f08727143)) - Highlander Paiva
- merge feature branch - ([111efaa](https://github.com/hvpaiva/tardis-cli/commit/111efaa84d6f63d09bcd1af61d1f19f571e2fbee)) - Highlander Paiva
- merge feature branch - ([6279b6a](https://github.com/hvpaiva/tardis-cli/commit/6279b6aeff0164945139c5c327f0552cfe722181)) - Highlander Paiva
- merge feature branch - ([6a8c0ee](https://github.com/hvpaiva/tardis-cli/commit/6a8c0ee9f48b5ff176df6cf32421f6d30d42bd45)) - Highlander Paiva
- merge feature branch - ([e044990](https://github.com/hvpaiva/tardis-cli/commit/e04499029d11b93c3fe07cf171b7625bf110bd3e)) - Highlander Paiva
- merge feature branch - ([0eb51d9](https://github.com/hvpaiva/tardis-cli/commit/0eb51d9ac825abf7702c605d2446813b210c1806)) - Highlander Paiva
- exclude internal docs from version control - ([f1d457b](https://github.com/hvpaiva/tardis-cli/commit/f1d457ba0d1cea777e490b298b4faa586e03f596)) - Highlander Paiva
- updated name to tardis-cli - ([b9caea7](https://github.com/hvpaiva/tardis-cli/commit/b9caea72ca123cb7677e1cf434962f5804dcdd9d)) - hvpaiva
- add comprehensive .gitattributes - ([f09bd07](https://github.com/hvpaiva/tardis-cli/commit/f09bd072056dcbb8928a8cafcc3dd314f9f6643c)) - hvpaiva
#### Style
- (**core**) apply rustfmt to parser and test files - ([9196bcc](https://github.com/hvpaiva/tardis-cli/commit/9196bcc67dcba1025292a76a95884d21a061ff75)) - Highlander Paiva
- (**core**) apply rustfmt formatting fixes - ([4ef0d65](https://github.com/hvpaiva/tardis-cli/commit/4ef0d65108d14131ed38f346d41707876c222227)) - Highlander Paiva

- - -


## [0.1.0] – 2025-06-25
### Added
- **Natural-language parsing** of date/time expressions via `human-date-parser`
  (e.g. `"next Monday at 09:00"`, `"in 2 hours"`, `"2025-12-31 23:59"`).
- **Custom output formats** (`--format/-f`) using `chrono` strftime syntax.
- **Named presets**: reusable formats declared under `[formats]` in
  `config.toml`.
- **Time-zone selection** (`--timezone/-t`) with full IANA/Olson database
  via `chrono-tz`; falls back to system local TZ if none given.
- **Reference clock override** (`--now`) for deterministic runs / tests
  (RFC 3339 input).
- **Config file** (`config.toml`) auto-created on first run:
  - Default `format` and `timezone`
  - Commented template for easy editing
  - Respects `XDG_CONFIG_HOME` or OS-specific config directory.
- **Environment-variable overrides**
  - `TARDIS_FORMAT`
  - `TARDIS_TIMEZONE`
- **Cross-platform shell completions** (bash, zsh, fish, PowerShell, elvish)
  and man-page generated at build time (`build.rs`).
- **Helpful error messages**
  - Unknown time-zone → `UnsupportedTimezone`
  - Empty/absent format → `MissingArgument`
  - Unparsable input → `InvalidDateFormat`
- **Extensive test suite**
  - Core logic, CLI merge rules, config loader, env-var guard.
- **Developer tooling**
  - `just` recipes (`lint_all`, `bench_quick`, `flamegraph`, etc.)
  - CI workflows for lint, test, audit, vet, and publish.
- **License**: MIT.

[0.1.0]: https://github.com/hvpaiva/tardis/releases/tag/v0.1.0
