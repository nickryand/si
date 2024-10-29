use audit_logs::AuditLogsStream;
use dal::{audit_logging, Component, DalContext, Schema};
use dal_test::{
    helpers::{update_attribute_value_for_component, ChangeSetTestHelpers},
    test,
};
use pending_events::PendingEventsStream;
use pretty_assertions_sorted::assert_eq;
use si_events::audit_log::AuditLogKind;

#[test]
async fn round_trip(ctx: &mut DalContext) {
    let schema = Schema::find_by_name(ctx, "swifty")
        .await
        .expect("could not perform find by name")
        .expect("schema not found by name");
    let schema_variant_id = schema
        .get_default_schema_variant_id(ctx)
        .await
        .expect("could not get default schema variant id")
        .expect("no default schema variant id found");

    // Create a component and commit. Mimic sdf by audit logging here.
    ctx.write_audit_log(AuditLogKind::CreateComponent)
        .await
        .expect("could not write audit log");
    let component = Component::new(ctx, "nyj despair club", schema_variant_id)
        .await
        .expect("could not create component");
    ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx)
        .await
        .expect("could not commit and update snapshot to visibility");

    // Collect the streams needed throughout the test.
    let (source_stream, destination_stream) = {
        let source_stream_wrapper = PendingEventsStream::get_or_create(ctx.jetstream_context())
            .await
            .expect("could not get or create pending events stream");
        let destination_stream_wrapper = AuditLogsStream::get_or_create(ctx.jetstream_context())
            .await
            .expect("could not get or create audit logs stream");
        let source_stream = source_stream_wrapper
            .stream()
            .await
            .expect("could not get inner stream");
        let destination_stream = destination_stream_wrapper
            .stream()
            .await
            .expect("could not get inner destination stream");
        (source_stream, destination_stream)
    };

    // Check that the streams look as we expect.
    assert_eq!(
        0,
        source_stream
            .get_info()
            .await
            .expect("could not get source stream info")
            .state
            .messages
    );
    assert_eq!(
        3,
        destination_stream
            .get_info()
            .await
            .expect("could not get destination stream info")
            .state
            .messages
    );

    // List all audit logs twice to ensure we don't consume/ack them. After that, check that they
    // look as we expect.
    let first_run_audit_logs = audit_logging::list(ctx)
        .await
        .expect("could not list audit logs");
    let second_run_audit_logs = audit_logging::list(ctx)
        .await
        .expect("could not list audit logs");
    assert_eq!(first_run_audit_logs, second_run_audit_logs);
    assert_eq!(3, first_run_audit_logs.len());

    // Update a property editor value and commit. Mimic sdf by audit logging here.
    ctx.write_audit_log(AuditLogKind::UpdatePropertyEditorValue)
        .await
        .expect("could not write audit log");
    update_attribute_value_for_component(
        ctx,
        component.id(),
        &["root", "domain", "name"],
        serde_json::json!["pain."],
    )
    .await
    .expect("could not update attribute value");
    ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx)
        .await
        .expect("could not commit and update snapshot to visibility");

    // Check that the streams look as we expect.
    assert_eq!(
        0,
        source_stream
            .get_info()
            .await
            .expect("could not get source stream info")
            .state
            .messages
    );
    assert_eq!(
        5,
        destination_stream
            .get_info()
            .await
            .expect("could not get destination stream info")
            .state
            .messages
    );

    // List all audit logs twice to ensure we don't consume/ack them. After that, check that they
    // look as we expect.
    let first_run_audit_logs = audit_logging::list(ctx)
        .await
        .expect("could not list audit logs");
    let second_run_audit_logs = audit_logging::list(ctx)
        .await
        .expect("could not list audit logs");
    assert_eq!(first_run_audit_logs, second_run_audit_logs);
    assert_eq!(5, first_run_audit_logs.len());

    // Delete a component and commit. Mimic sdf by audit logging here.
    ctx.write_audit_log(AuditLogKind::DeleteComponent)
        .await
        .expect("could not write audit log");
    assert!(component
        .delete(ctx)
        .await
        .expect("unable to delete component")
        .is_none());
    ChangeSetTestHelpers::commit_and_update_snapshot_to_visibility(ctx)
        .await
        .expect("could not commit and update snapshot to visibility");

    // Check that the streams look as we expect.
    assert_eq!(
        0,
        source_stream
            .get_info()
            .await
            .expect("could not get source stream info")
            .state
            .messages
    );
    assert_eq!(
        6,
        destination_stream
            .get_info()
            .await
            .expect("could not get destination stream info")
            .state
            .messages
    );

    // List all audit logs twice to ensure we don't consume/ack them. After that, check that they
    // look as we expect.
    let first_run_audit_logs = audit_logging::list(ctx)
        .await
        .expect("could not list audit logs");
    let second_run_audit_logs = audit_logging::list(ctx)
        .await
        .expect("could not list audit logs");
    assert_eq!(first_run_audit_logs, second_run_audit_logs);
    assert_eq!(6, first_run_audit_logs.len());
}
