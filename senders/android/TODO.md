# Android Sender Migration TODO

## Goal
Migrate the old `agdk-eframe/src` command API + media-graph control model into
`senders/android`, while keeping existing Android sender capture/casting flows working.

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
- [ ] Run Android device smoke test with JNI command ingress.
- [ ] Validate compatibility against legacy controller client scripts.

## Risks / Follow-ups
- [ ] Replace metadata-only node internals with real GStreamer pipeline behavior from old `domain/nodes/*`.
- [ ] Add scheduled execution and EOS behavior parity.
- [ ] Add remote transport endpoint for command ingress if needed (`/command` equivalent).
- [ ] Map all old mixer/source slot property semantics to runtime behavior.
