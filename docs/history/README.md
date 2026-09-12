# Historical engineering records

These are dated snapshots, not current setup instructions or release approvals.
Features described as planned, tested or blocked apply to the named revision.
Local artifact paths may now be inside the evidence archive created by cleanup.

| Record | Purpose |
| --- | --- |
| [Original requirements](PRODUCT_REQUIREMENTS.md) | Initial product goals, including goals not implemented |
| [Upstream audit](UPSTREAM_AUDIT.md) | Initial repository/license/backend investigation |
| [BLE HID comparison](BLE_HID_COMPARISON.md) | Protocol compatibility investigation |
| [Native UI milestone](NATIVE_UI_MILESTONE.md) | Initial UI/Raw Input plan and evidence |
| [Phase 5 hardening](HARDENING_PHASE5.md) | Earlier exact-binary validation gate |
| [UI polish validation](UI_POLISH_VALIDATION.md) | Compact UI, Fluent SVG and idle measurements |
| [Performance audit](PERFORMANCE_HARDENING_AUDIT.md) | Earlier code/performance baseline |
| [Dependency audit](DEPENDENCY_AUDIT.md) | Dependency graph before later maintenance |

Use [current validation](../VALIDATION.md), [architecture](../ARCHITECTURE.md) and
the [engineering log](../ENGINEERING_LOG.md) for current context. Failed absolute
touch research remains in Git at `rollback/pre-native-ui-raw-input-20260911`;
it is not a production control backend.
