# Cross-fixture painting consistency (2026-11-06)

Pairs of fixtures in `src/test/data/diffs/` whose hand paintings handle the same kind of change
differently. A fixture with a single painting ("Only one solution", or only `Minimal`) is
held to both presets (`paintings_for_mode`), so in this table its painting counts as its `Full`
painting too. `a` -> `b` names the change, and `X`/`Y` is what is painted on each side.

How these were found: every painted fixture was line-diffed; each one-line-to-one-line edit was
classified (version/number, identifier, prose/comment word, indentation, internal whitespace) and
its painted extent recorded per preset; separately, every painted `Insert`/`Delete` run inside a
line was checked for whether it takes the adjacent whitespace. Groups of the same change that are
painted differently were then checked by hand. Not covered: edits that span several lines, and
tree-mapping-only differences that the paintings do not show.

| Fixture 1 | Fixture 2 | Difference |
|---|---|---|
| `php-wordpress-wordpress-version-update` (also `-2`, `-3`) | `go-gin-gonic-gin-update-version-string-4` | **Version, Full.** WordPress `61516` -> `61517` paints the whole numeric component (`61516`/`61517`); gin `v1.11.0` -> `v1.12.0` paints only the changed digit (`1`/`2`). |
| `go-gin-constant-change` | `cpp-libreoffice-interesting-complex-case-with-comments` | **Number, Full** (both have an explicit `Full` painting). `14` -> `15` paints the whole number; `43` -> `44` paints only `3`/`4`. |
| `go-gohugoio-hugo-add-and-upadate-list-items` (also `-update-and-add-list-items`, `-update-and-add-some-values`) | `python-ytdl-org-youtube-dl-version-update` | **Number/version, Full.** `Minor: 148` -> `149` paints `148`/`149`; `2021.04.07` -> `2021.04.17` paints only `0`/`1`. |
| `cpp-mongodb-mongo-test-changes` | `python-odoo-odoo-version` | **Number, Full.** `10001` -> `10002` paints the whole literal; `Odoo 20` -> `Odoo 22` paints only `0`/`2`. |
| `kotlin-mozilla-firefox-firefox-version` | `csharp-jellyfin-jellyfin-update-version-string` | **Version, Full.** `150.20260317050357` -> `…20050245` paints the whole digit run after the dot; `10.11.0` -> `10.12.0` paints only `1`/`2`. |
| `python-pandas-dev-pandas-update-comment-one-byte` | `c-freeciv-freeciv-null-to-nullptr` | **Comment word, Full** (both explicit). `a` -> `an` paints the whole word; `initialise` -> `Initialise` (and `test` -> `Test`) paints only `i`/`I`. |
| `python-pandas-dev-pandas-update-one-letter` | `lua-awesomewm-awesome-fix-one-byte-typo-in-comment` | **Comment typo, Full.** `multindex` -> `multiindex` paints the whole word; `bottons` -> `buttons` paints only `o`/`u`. |
| `cpp-opencv-opencv-fix-dimention-dimension-typo-in-two-comments` (also `cpp-paddlepaddle-paddleocr-childs-to-children-rename`) | `c-sqlite-sqlite-fix-format-string-typo` (also `shellscript-nvm-sh-nvm-tiny-change-lowercase-to-uppercase`) | **Typo, Full.** `dimentions` -> `dimensions` paints the whole word; `validYMS` -> `validYMD` paints only `S`/`D` (`Github` -> `GitHub`: only `h`/`H`). |
| `cpp-opencv-opencv-fix-dimention-dimension-typo-in-two-comments`, row 1370 | the same fixture, rows 1369 and 1377 | **Missed occurrence.** The same `dimentions` -> `dimensions` fix appears on three rows; row 1370 is unpainted in both presets. |
| `java-defects4j-cli-36-optiongroup` | `java-defects4j-cli-33-helpformatter` | **Identifier gains a prefix/suffix, Minimal.** `HashMap` -> `LinkedHashMap` paints only `Linked` as Insert; `renderWrappedText` -> `renderWrappedTextBlock` paints Update over both whole identifiers. |
| `c-microsoft-terminal-change-import-path` (also `csharp-waf-csharprepl-update-string-URL`) | `csharp-sonarr-sonarr-many-updates-with-one-insert` | **Inserted characters inside a token, Minimal.** `json.h` -> `json/json.h` paints only `json/` as Insert (`http` -> `https`: only `s`); `IDecisionEngineSpecification` -> `IDownloadDecisionEngineSpecification` paints Update over both whole identifiers. |
| `rust-tauri-apps-tauri-add-to-list` | `rust-tauri-apps-tauri-mostly-delete` | **Separator space, Full.** In a `use {…}` list, the inserted ` IosSettings,` takes the space before it; the deleted `InvokeResponder,` takes no space. |
| `rust-rust-lang-rust-add-key-to-function-arguments-and-call-sites` | `java-defects4j-mockito-19-namebasedcandidatefilter` | **Separator space, Full.** An argument inserted mid-list: ` key,` takes the leading space; `fields,` (`mocks, field, fields, fieldInstance`) takes none. |
| `yaml-n8n-io-n8n-add-performance-check-to-ci-config` | `ruby-mastodon-mastodon-normal-change` | **Separator space, Full.** List element ` performance,` takes the leading space; deleted `:position,` takes none. |
| `python-django-django-tiny-change` | `rust-vercel-next-delete-only` | **Separator space, Full.** Inserted import ` constant_time_compare,` takes the leading space; deleted `new_turbo_bincode_encoder,` takes none. |
| `rust-tauri-apps-tauri-insert-async-modifier` (also `cpp-libreoffice-add-const`: `const `) | `php-nextcloud-server-add-readonly` (also `php-nextcloud-server-add-a-few-types`, `csharp-radarr-radarr-small-change`: `override`) | **Inserted modifier, Full.** ` async` takes an adjacent space (`const ` takes the one after); `readonly` and `override` take no space. |
| `json-immich-app-immich-small-change` (also `vimscript-neovim-neovim-changes-in-string`: ` -I`) | `shellscript-ansible-ansible-a-small-add` | **Leading space, Minimal.** Inserted flag ` --cache` includes the space before it; inserted flag `--unified` does not. |
| `java-defects4j-jsoup-56-documenttype` (also `go-ollama-ollama-interesting-comment-reduces-by-one-line`: ` if backoff == 0`) | `java-add-interface` | **Leading space, Minimal.** Deleted ` PUBLIC` includes the space before it; inserted `implements Shape` (spaces on both sides) takes none, as do about 290 other Minimal runs. |
| `python-api-change` (also `html-pandas-dev-pandas-release-banner-update`: ` released`) | `java-add-interface` | **Leading space, Minimal.** Docstring words ` with improved error handling` and ` with type hints` include the leading space, although the same fixture's `-> Optional[…]` does not. |
| `swift-swiftlang-swift-target-comment` (also `-target-comment-2`, `-add-target-comment`, and `javascript-facebook-react-update-comment-only`: ` @outputMode:"lint"`) | `shellscript-ansible-ansible-a-small-add` | **Leading space, single painting** (so Minimal too). The inserted ` %target-embedded-posix-shim` includes its leading space. |
| `vimscript-neovim-neovim-small-change-2` | `java-add-interface` | **Trailing space, Minimal.** Inserted `\| ` includes the space after it. |
| `c-openssl-openssl-whitespace-only` | `c-openssl-openssl-format-only-change` | **Re-indented continuation line.** `sizeof(…)` moved from column 34 to column 12 is unpainted (single painting, so Full too); `int mdnid)` moved from column 55 to column 4 is painted under Full: the removed indentation as Delete and the line as Move. |
| `shellscript-stgpetrovic-stacuist-pure-add` | `c-openssl-openssl-format-only-change` | **Indentation removed, Full.** ` # Compile the app…` -> `# Compile…` is unpainted under Full; format-only paints its removed indentation as Delete. |
| `go-gin-gonic-gin-one-space-removed-in-a-comment` (also `go-gin-gonic-gin-whitespace-in-comment`) | `c-ffmpeg-added-typedef-to-enum` | **Internal whitespace removed.** `Team.  All` -> `Team. All` paints the removed space as Delete (single painting, so Minimal too); `unused[4];    /*` -> `unused[4]; /*` leaves the removed spaces unpainted. |
| `go-gin-gonic-gin-one-space-removed-in-a-comment` | `go-jesseduffield-lazygit-update-in-ast-delete-in-comment` | **Internal whitespace, Minimal.** gin paints the removed space under Minimal; lazygit `//    assert` -> `//<tab>assert` paints the removed spaces only under Full. |
| `c-openssl-openssl-whitepsace-only` | `c-openssl-openssl-format-only-change` | **Alignment whitespace shrunk.** `NULL,<28 spaces>/* opener */` -> `NULL, /* opener */` paints the whole line as Update on both sides; `MD_with_RSA_CASE(…)<spaces>\` paints only the removed spaces (Delete, Full) and the rest as Move. |
| `csharp-jellyfin-jellyfin-add-only` (and 27 other single-painting fixtures, all checked ones named `Minimal`, e.g. `csharp-sonarr-sonarr-pure-add`, `python-django-django-add-test`, `cpp-protocolbuffers-protobuf-add-tests`, `java-defects4j-jsoup-36-datautil`) | `csharp-jellyfin-jellyfin-add-a-loop` | **Indentation of whole added/removed lines under Full.** These have a single painting, which Full is graded against too, so indentation of wholly inserted lines is unpainted under Full; all 597 explicit Full paintings paint it. |
| `html-apache-echarts-raw-text-contains-code` | `go-ollama-ollama-interesting-commenting-out-code` | **Commenting code out or back in.** `show: false,` -> `// show: false,` and three `// x: 0,` -> `x: 0,` lines are unpainted in both presets; ollama paints the inserted `//` (Full: `// `) as Insert. |

## Consistent (checked, no differences found)

- Indentation of wholly inserted/deleted lines: every explicit `Full` painting paints it (597), and
  every `Minimal` painting leaves it alone (596).
- Version bumps in single-painting fixtures (gin, jekyll, nvm, puppeteer, vercel, tauri, mui,
  postgres copyright years): all paint the span from the first to the last differing character.
- `Minimal` keeps a list separator with the element it belongs to (`fields,`, `, true`).
- Identifier renames under explicit `Full`: whole identifier.
