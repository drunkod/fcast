# Android Sender Migration TODO

## Goal
Migrate the old `old-version/rust-android-examples/agdk-eframe/src` command API + media-graph control model into
`senders/android/src`, while keeping existing Android sender capture/casting flows working.

## Scope
- In scope:
  - Command protocol (`Command`, `CommandResult`, node info, control points).
  - Graph runtime entrypoints.
  - `NodeManager` command dispatcher and node/link state model.
  - Integration points in `senders/android/src/lib.rs` for startup/shutdown and JNI command ingress.
- Out of scope (for this migration pass):
  - Porting old egui UI.
  - Porting old Actix HTTP/WebSocket server.
  - Full GStreamer pipeline parity for every old node behavior.

## Phase 0: Baseline
- [x] Confirm `senders/android` crate structure and current `lib.rs` entrypoints.
- [x] Confirm old protocol/source of truth in `old-version/.../application/common/controller.rs`.

## Phase 1: Protocol Port
- [x] Add `senders/android/src/migration/protocol.rs`.
- [x] Port command enums/structs and message wrappers.
- [x] Keep serde shape compatible with old lowercase command protocol.

## Phase 2: Graph Core Port
- [x] Add `senders/android/src/migration/node_manager.rs`.
- [x] Add in-memory node/link model and command dispatch.
- [x] Implement command handlers:
  - [x] `CreateVideoGenerator`
  - [x] `CreateSource`
  - [x] `CreateDestination`
  - [x] `CreateMixer`
  - [x] `Connect`
  - [x] `Disconnect`
  - [x] `Start`
  - [x] `Reschedule`
  - [x] `Remove`
  - [x] `GetInfo`
  - [x] `AddControlPoint`
  - [x] `RemoveControlPoint`

## Phase 3: Node Modules
- [x] Add `senders/android/src/migration/nodes/source.rs`.
- [x] Add `senders/android/src/migration/nodes/destination.rs`.
- [x] Add `senders/android/src/migration/nodes/mixer.rs`.
- [x] Add `senders/android/src/migration/nodes/video_generator.rs`.
- [x] Add `senders/android/src/migration/nodes/mod.rs`.

## Phase 4: Runtime Integration
- [x] Add `senders/android/src/migration/runtime.rs`.
- [x] Add lifecycle methods:
  - [x] `start_graph_runtime`
  - [x] `shutdown_graph_runtime`
  - [x] `handle_command`
  - [x] `handle_controller_message`
  - [x] `handle_command_json`
- [x] Hook runtime startup into `Application::run_event_loop`.
- [x] Hook runtime shutdown on event loop exit.
- [x] Add JNI entrypoint for command JSON ingestion and response JSON.

## Phase 5: Validation
- [x] Add focused unit tests for command flow in migration runtime.
- [x] Add focused node-level scheduling test coverage for migration `SourceNode`.
- [x] Add focused node-level scheduling test coverage for migration `DestinationNode`.
- [x] Add focused node-level scheduling test coverage for migration `MixerNode`.
- [ ] Run Android device smoke test with JNI command ingress.
- [x] Create Android UI path to trigger and view smoke test results.
- [x] Validate compatibility against legacy controller client scripts.

## Phase 6: True Media Parity (In Progress)
- [x] Port old node scheduling/state behavior into new `nodes/*` runtime models.
- [x] Port mixer control-point timeline behavior (`set` + numeric `interpolate`) and slot application.
- [x] Port destination family-specific pipeline profile modeling and start precondition checks.
- [x] Wire Java helper path to call `nativeGraphCommand` and parse `success`/`error`/`info` responses.
- [x] Add concrete Java call site (`getinfo`) in app flow and parse/log response.
- [x] Add concrete Java smoke sequence (`createvideogenerator` + `createmixer` + `connect` + `start` + `getinfo`, with cleanup).
- [x] Start live GStreamer execution in migration runtime for `VideoGeneratorNode` (create/link/state transitions).
- [x] Start live GStreamer execution in migration runtime for `SourceNode` (fallbacksrc/uridecodebin + dynamic pad linking + state transitions).
- [x] Start live GStreamer execution in migration runtime for `DestinationNode` (family pipelines + appsrc wiring + state transitions).
- [x] Start live GStreamer execution in migration runtime for `MixerNode` (compositor/audiomixer + slot appsrc wiring + state transitions).
- [x] Replace modeled node profiles with live GStreamer pipeline execution.
- [x] Port EOS and async bus transition behavior from old `domain/nodes/*`.

## Risks / Follow-ups
- [ ] Explore old server behavior documented in `old-version/rust-android-examples/explain.md` and map remaining `/command` parity gaps.
- [x] Replace metadata-only node internals with richer parity runtime models from old `domain/nodes/*`.
- [x] Replace runtime models with real GStreamer pipeline behavior from old `domain/nodes/*`.
- [x] Wire migrated node producer/consumer media flow end-to-end (`appsink` -> `appsrc`) for true data-path parity.
- [x] Add scheduled execution and EOS behavior parity.
- [x] Add remote transport endpoint for command ingress if needed (`/command` equivalent).
- [x] Map remaining old mixer/source slot property semantics to runtime behavior.
