# Cargo dependency audit

Generated from locked Windows x64 metadata before implementation changes. No crates removed in this audit. Runtime means linked/reachable from the app, not that an optional backend is eagerly started. Resolved features and dependency edges are retained in the local baseline metadata.

| Dependency | Role | Why present / immediate users | Resolved feature count | Default features requested by a parent | Decision |
|---|---|---|---:|---|---|
| atomic-waker 1.1.2 | runtime | Shared implementation utility; via hyper | 0 | True | KEEP pending measured benefit |
| base64 0.22.1 | runtime | Shared implementation utility; via hyper-util, reqwest | 3 | True | KEEP pending measured benefit |
| bitflags 2.13.2 | runtime | Shared implementation utility; via tower-http | 0 | True | KEEP pending measured benefit |
| bitstream-io 2.6.0 | runtime | H.264 metadata/parser support; via h264-reader | 2 | True | KEEP pending measured benefit |
| bytes 1.12.1 | runtime | Shared implementation utility; via http, http-body, http-body-util, hyper, hyper-util, reqwest, tower-http | 2 | True | KEEP pending measured benefit |
| cc 1.2.56 | build | Native build/tool discovery; via imirror-native-core | 0 | True | KEEP pending measured benefit |
| crossbeam-channel 0.5.15 | runtime | Bounded thread channels; via imirror | 2 | True | KEEP pending measured benefit |
| crossbeam-utils 0.8.23 | runtime | Bounded thread channels; via crossbeam-channel | 1 | False | KEEP pending measured benefit |
| displaydoc 0.2.7 | build | URL/Unicode support for the HTTP stack; via icu_collections, icu_locale_core, icu_properties, icu_provider, tinystr, zerotrie | 0 | False | KEEP pending measured benefit |
| find-msvc-tools 0.1.12 | build | Native build/tool discovery; via cc | 0 | True | KEEP pending measured benefit |
| form_urlencoded 1.2.2 | runtime | URL/Unicode support for the HTTP stack; via serde_urlencoded, url | 3 | True | KEEP pending measured benefit |
| four-cc 0.4.0 | runtime | H.264 metadata/parser support; via mp4ra-rust | 2 | True | KEEP pending measured benefit |
| futures-channel 0.3.34 | runtime | Native HTTP/async I/O or UDP transport; via hyper, hyper-util, reqwest | 5 | True | KEEP pending measured benefit |
| futures-core 0.3.34 | runtime | Native HTTP/async I/O or UDP transport; via futures-channel, futures-util, http-body-util, hyper, reqwest, sync_wrapper, tower | 3 | True | KEEP pending measured benefit |
| futures-io 0.3.34 | runtime | Native HTTP/async I/O or UDP transport; via futures-util | 1 | False | KEEP pending measured benefit |
| futures-sink 0.3.34 | runtime | Native HTTP/async I/O or UDP transport; via futures-channel, futures-util | 0 | False | KEEP pending measured benefit |
| futures-task 0.3.34 | runtime | Native HTTP/async I/O or UDP transport; via futures-util | 2 | False | KEEP pending measured benefit |
| futures-util 0.3.34 | runtime | Native HTTP/async I/O or UDP transport; via hyper-util, reqwest, tower, tower-http | 8 | True | KEEP pending measured benefit |
| h264-reader 0.8.0 | runtime | H.264 metadata/parser support; via imirror-video-core | 0 | True | KEEP pending measured benefit |
| hex-slice 0.1.4 | runtime | H.264 metadata/parser support; via h264-reader | 0 | True | KEEP pending measured benefit |
| http 1.5.0 | runtime | Native HTTP/async I/O or UDP transport; via http-body, http-body-util, hyper, hyper-util, reqwest, tower-http | 2 | True | KEEP pending measured benefit |
| httparse 1.10.1 | runtime | Native HTTP/async I/O or UDP transport; via hyper | 2 | True | KEEP pending measured benefit |
| http-body 1.1.0 | runtime | Native HTTP/async I/O or UDP transport; via http-body-util, hyper, hyper-util, reqwest, tower-http | 0 | True | KEEP pending measured benefit |
| http-body-util 0.1.5 | runtime | Native HTTP/async I/O or UDP transport; via reqwest | 1 | True | KEEP pending measured benefit |
| hyper 1.11.1 | runtime | Native HTTP/async I/O or UDP transport; via hyper-util, reqwest | 3 | True | KEEP pending measured benefit |
| hyper-util 0.1.20 | runtime | Native HTTP/async I/O or UDP transport; via reqwest | 6 | True | KEEP pending measured benefit |
| icu_collections 2.3.0 | runtime | URL/Unicode support for the HTTP stack; via icu_normalizer, icu_properties | 0 | False | KEEP pending measured benefit |
| icu_locale_core 2.3.0 | runtime | URL/Unicode support for the HTTP stack; via icu_properties, icu_provider | 1 | False | KEEP pending measured benefit |
| icu_normalizer 2.3.0 | runtime | URL/Unicode support for the HTTP stack; via idna_adapter | 1 | False | KEEP pending measured benefit |
| icu_normalizer_data 2.3.0 | runtime | URL/Unicode support for the HTTP stack; via icu_normalizer | 0 | False | KEEP pending measured benefit |
| icu_properties 2.3.0 | runtime | URL/Unicode support for the HTTP stack; via icu_normalizer, idna_adapter | 1 | False | KEEP pending measured benefit |
| icu_properties_data 2.3.0 | runtime | URL/Unicode support for the HTTP stack; via icu_properties | 0 | False | KEEP pending measured benefit |
| icu_provider 2.3.1 | runtime | URL/Unicode support for the HTTP stack; via icu_normalizer, icu_properties | 1 | False | KEEP pending measured benefit |
| idna 1.1.0 | runtime | URL/Unicode support for the HTTP stack; via url | 3 | False | KEEP pending measured benefit |
| idna_adapter 1.2.2 | runtime | URL/Unicode support for the HTTP stack; via idna | 1 | True | KEEP pending measured benefit |
| ipnet 2.12.2 | runtime | Shared implementation utility; via hyper-util | 2 | True | KEEP pending measured benefit |
| itoa 1.0.18 | runtime | Shared implementation utility; via http, hyper, serde_json, serde_urlencoded | 0 | True | KEEP pending measured benefit |
| libc 0.2.189 | runtime | Shared implementation utility; via hyper-util | 2 | True | KEEP pending measured benefit |
| litemap 0.8.3 | runtime | URL/Unicode support for the HTTP stack; via icu_locale_core | 0 | False | KEEP pending measured benefit |
| log 0.4.34 | runtime | Shared implementation utility; via h264-reader, reqwest | 0 | True | KEEP pending measured benefit |
| memchr 2.8.3 | runtime | Shared implementation utility; via futures-util, h264-reader, serde_json | 3 | True | KEEP pending measured benefit |
| mio 1.2.3 | runtime | Native HTTP/async I/O or UDP transport; via tokio | 3 | False | KEEP pending measured benefit |
| mp4ra-rust 0.3.0 | runtime | H.264 metadata/parser support; via rfc6381-codec | 0 | True | KEEP pending measured benefit |
| mpeg4-audio-const 0.2.0 | runtime | H.264 metadata/parser support; via rfc6381-codec | 0 | True | KEEP pending measured benefit |
| once_cell 1.21.4 | runtime | Shared implementation utility; via tracing-core | 4 | True | KEEP pending measured benefit |
| percent-encoding 2.3.2 | runtime | URL/Unicode support for the HTTP stack; via form_urlencoded, hyper-util, reqwest, url | 3 | True | KEEP pending measured benefit |
| pin-project-lite 0.2.17 | runtime | Shared implementation utility; via futures-util, http-body-util, hyper, hyper-util, reqwest, tokio, tower, tower-http, tracing | 0 | True | KEEP pending measured benefit |
| potential_utf 0.1.6 | runtime | URL/Unicode support for the HTTP stack; via icu_collections | 1 | False | KEEP pending measured benefit |
| proc-macro2 1.0.107 | build | Serialization, typed errors or code generation; via displaydoc, quote, serde_derive, syn, synstructure, thiserror-impl, tracing-attributes, windows-implement, windows-interface, yoke-derive, zerofrom-derive, zerovec-derive | 2 | True | KEEP pending measured benefit |
| quote 1.0.47 | build | Serialization, typed errors or code generation; via displaydoc, serde_derive, syn, synstructure, thiserror-impl, tracing-attributes, windows-implement, windows-interface, yoke-derive, zerofrom-derive, zerovec-derive | 2 | True | KEEP pending measured benefit |
| reqwest 0.12.28 | runtime | Native HTTP/async I/O or UDP transport; via imirror-input-wda | 2 | False | KEEP pending measured benefit |
| rfc6381-codec 0.2.0 | runtime | H.264 metadata/parser support; via h264-reader | 0 | True | KEEP pending measured benefit |
| ryu 1.0.23 | runtime | Shared implementation utility; via serde_urlencoded | 0 | True | KEEP pending measured benefit |
| serde 1.0.228 | runtime | Serialization, typed errors or code generation; via imirror-device, imirror-input-ble, imirror-input-core, imirror-native-core, imirror-platform-windows, reqwest, serde_urlencoded, url | 4 | True | KEEP pending measured benefit |
| serde_core 1.0.228 | runtime | Serialization, typed errors or code generation; via serde, serde_json | 2 | False | KEEP pending measured benefit |
| serde_derive 1.0.228 | build | Serialization, typed errors or code generation; via serde | 1 | True | KEEP pending measured benefit |
| serde_json 1.0.151 | runtime | Serialization, typed errors or code generation; via imirror, imirror-device, imirror-input-wda, reqwest | 2 | True | KEEP pending measured benefit |
| serde_urlencoded 0.7.1 | runtime | Serialization, typed errors or code generation; via reqwest | 0 | True | KEEP pending measured benefit |
| shlex 1.3.0 | build | Native build/tool discovery; via cc | 2 | True | KEEP pending measured benefit |
| slab 0.4.12 | runtime | Shared implementation utility; via futures-util | 1 | False | KEEP pending measured benefit |
| smallvec 1.16.0 | runtime | Shared implementation utility; via hyper, icu_normalizer, idna | 2 | True | KEEP pending measured benefit |
| socket2 0.6.5 | runtime | Native HTTP/async I/O or UDP transport; via hyper-util, imirror-video-airplay, tokio | 1 | True | KEEP pending measured benefit |
| stable_deref_trait 1.2.1 | runtime | URL/Unicode support for the HTTP stack; via yoke | 0 | False | KEEP pending measured benefit |
| syn 3.0.5 | build | Serialization, typed errors or code generation; via displaydoc, zerovec-derive | 10 | True | KEEP pending measured benefit |
| syn 2.0.119 | build | Serialization, typed errors or code generation; via serde_derive, synstructure, thiserror-impl, tracing-attributes, windows-implement, windows-interface, yoke-derive, zerofrom-derive | 11 | True | KEEP pending measured benefit |
| sync_wrapper 1.0.2 | runtime | Serialization, typed errors or code generation; via reqwest, tower | 2 | True | KEEP pending measured benefit |
| synstructure 0.13.2 | build | Serialization, typed errors or code generation; via yoke-derive, zerofrom-derive | 2 | True | KEEP pending measured benefit |
| thiserror 2.0.18 | runtime | Serialization, typed errors or code generation; via imirror-coordinate-map, imirror-decoder, imirror-device, imirror-input-ble, imirror-input-wda, imirror-native-core, imirror-video-airplay, imirror-video-core | 2 | True | KEEP pending measured benefit |
| thiserror-impl 2.0.18 | build | Serialization, typed errors or code generation; via thiserror | 0 | True | KEEP pending measured benefit |
| tinystr 0.8.4 | runtime | URL/Unicode support for the HTTP stack; via icu_locale_core | 1 | False | KEEP pending measured benefit |
| tokio 1.53.1 | runtime | Native HTTP/async I/O or UDP transport; via hyper, hyper-util, reqwest, tower | 9 | True | KEEP pending measured benefit |
| tower 0.5.3 | runtime | Native HTTP/async I/O or UDP transport; via reqwest, tower-http | 8 | True | KEEP pending measured benefit |
| tower-http 0.6.11 | runtime | Native HTTP/async I/O or UDP transport; via reqwest | 3 | False | KEEP pending measured benefit |
| tower-layer 0.3.3 | runtime | Native HTTP/async I/O or UDP transport; via tower, tower-http | 0 | True | KEEP pending measured benefit |
| tower-service 0.3.3 | runtime | Native HTTP/async I/O or UDP transport; via hyper-util, reqwest, tower, tower-http | 0 | True | KEEP pending measured benefit |
| tracing 0.1.44 | runtime | Structured diagnostic events; via hyper-util, imirror, imirror-video-airplay | 4 | True | KEEP pending measured benefit |
| tracing-attributes 0.1.31 | build | Structured diagnostic events; via tracing | 0 | True | KEEP pending measured benefit |
| tracing-core 0.1.36 | runtime | Structured diagnostic events; via tracing | 2 | False | KEEP pending measured benefit |
| try-lock 0.2.5 | runtime | Native HTTP/async I/O or UDP transport; via want | 0 | True | KEEP pending measured benefit |
| unicode-ident 1.0.24 | build | Serialization, typed errors or code generation; via proc-macro2, syn | 0 | True | KEEP pending measured benefit |
| url 2.5.8 | runtime | URL/Unicode support for the HTTP stack; via reqwest, tower-http | 2 | True | KEEP pending measured benefit |
| utf8_iter 1.0.4 | runtime | URL/Unicode support for the HTTP stack; via icu_collections, idna | 0 | True | KEEP pending measured benefit |
| want 0.3.1 | runtime | Native HTTP/async I/O or UDP transport; via hyper | 0 | True | KEEP pending measured benefit |
| windows 0.61.3 | runtime | Windows/WinRT bindings; via imirror, imirror-input-ble, imirror-platform-windows, imirror-video-airplay | 45 | True | KEEP pending measured benefit |
| windows-collections 0.2.0 | runtime | Windows/WinRT bindings; via windows | 0 | False | KEEP pending measured benefit |
| windows-core 0.61.2 | runtime | Windows/WinRT bindings; via windows, windows-collections, windows-future, windows-numerics | 1 | False | KEEP pending measured benefit |
| windows-future 0.2.1 | runtime | Windows/WinRT bindings; via imirror-input-ble, windows | 2 | True | KEEP pending measured benefit |
| windows-implement 0.60.2 | build | Windows/WinRT bindings; via windows-core | 0 | False | KEEP pending measured benefit |
| windows-interface 0.59.3 | build | Windows/WinRT bindings; via windows-core | 0 | False | KEEP pending measured benefit |
| windows-link 0.2.1 | runtime | Windows/WinRT bindings; via windows-sys | 0 | False | KEEP pending measured benefit |
| windows-link 0.1.3 | runtime | Windows/WinRT bindings; via windows, windows-core, windows-future, windows-numerics, windows-result, windows-strings, windows-threading | 0 | False | KEEP pending measured benefit |
| windows-numerics 0.2.0 | runtime | Windows/WinRT bindings; via windows | 0 | False | KEEP pending measured benefit |
| windows-result 0.3.4 | runtime | Windows/WinRT bindings; via windows-core | 1 | False | KEEP pending measured benefit |
| windows-strings 0.4.2 | runtime | Windows/WinRT bindings; via windows-core | 1 | False | KEEP pending measured benefit |
| windows-sys 0.61.2 | runtime | Windows/WinRT bindings; via mio, socket2, tokio | 20 | True | KEEP pending measured benefit |
| windows-threading 0.1.0 | runtime | Windows/WinRT bindings; via windows-future | 0 | False | KEEP pending measured benefit |
| writeable 0.6.4 | runtime | URL/Unicode support for the HTTP stack; via icu_locale_core, icu_provider | 0 | False | KEEP pending measured benefit |
| yoke 0.8.3 | runtime | URL/Unicode support for the HTTP stack; via icu_collections, icu_provider, zerotrie, zerovec | 2 | False | KEEP pending measured benefit |
| yoke-derive 0.8.2 | build | URL/Unicode support for the HTTP stack; via yoke | 0 | False | KEEP pending measured benefit |
| zerofrom 0.1.8 | runtime | URL/Unicode support for the HTTP stack; via icu_collections, icu_provider, yoke, zerotrie, zerovec | 1 | False | KEEP pending measured benefit |
| zerofrom-derive 0.1.7 | build | URL/Unicode support for the HTTP stack; via zerofrom | 0 | False | KEEP pending measured benefit |
| zerotrie 0.2.5 | runtime | URL/Unicode support for the HTTP stack; via icu_properties, icu_provider | 2 | False | KEEP pending measured benefit |
| zerovec 0.11.8 | runtime | URL/Unicode support for the HTTP stack; via icu_collections, icu_locale_core, icu_normalizer, icu_properties, icu_provider, potential_utf, tinystr | 2 | False | KEEP pending measured benefit |
| zerovec-derive 0.11.6 | build | URL/Unicode support for the HTTP stack; via zerovec | 0 | False | KEEP pending measured benefit |
| zmij 1.0.23 | runtime | Shared implementation utility; via serde_json | 0 | True | KEEP pending measured benefit |

Direct dependency decisions: reqwest already uses default-features=false with blocking/json. h264-reader is retained to avoid rewriting a mature parser. windows/windows-future implement real native API paths. cc is build-only. No native GStreamer/FFmpeg plugin removal is justified before wireless and clean-machine validation.

A default-feature request somewhere in the graph is not proof that it can safely be disabled. Inspect the requesting edge and actual API use before changing it. Proc-macro/build utilities do not become end-user DLL requirements.
