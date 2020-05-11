use crate::agent::mqtt::{Message, MqttClient};
use crate::entity_event::EntityEvent;
use crate::error::CeaResult;
use futures::compat::Future01CompatExt;
use si_data::uuid_string;
use si_settings::Settings;

pub use tracing::{debug, debug_span};
pub use tracing_futures::Instrument as _;

#[derive(Debug, Clone)]
pub struct AgentClient {
    pub mqtt: MqttClient,
}

impl AgentClient {
    pub async fn new(name: &str, settings: &Settings) -> CeaResult<AgentClient> {
        // Create a client & define connect options
        let client_id = format!("agent_client:{}:{}", name, uuid_string());

        let mqtt = MqttClient::new()
            .server_uri(settings.vernemq_server_uri().as_ref())
            .client_id(client_id.as_ref())
            .persistence(false)
            .finalize();
        mqtt.default_connect().await?;

        Ok(AgentClient { mqtt })
    }

    pub async fn dispatch(&self, entity_event: &impl EntityEvent) -> CeaResult<()> {
        async {
            entity_event.validate_input_entity()?;
            entity_event.validate_action_name()?;
            self.send(entity_event).await?;
            Ok(())
        }
        .instrument(debug_span!("async_client_dispatch", ?entity_event))
        .await
    }

    // Eventually, we need to be able to dispatch to an individual agent id, so
    // that people can run specific agents for their billing account. We can
    // do that by just putting it in the EntityEvent stuct, and if it is
    // filled in, we use it.
    fn generate_topic(&self, entity_event: &impl EntityEvent) -> CeaResult<String> {
        let topic = format!(
            "{}/{}/{}/{}/{}/{}/{}/{}/{}",
            entity_event.billing_account_id()?,
            entity_event.organization_id()?,
            entity_event.workspace_id()?,
            entity_event.integration_id()?,
            entity_event.integration_service_id()?,
            entity_event.entity_id()?,
            "action",
            entity_event.action_name()?,
            entity_event.id()?,
        );

        Ok(topic)
    }

    pub async fn send(&self, entity_event: &impl EntityEvent) -> CeaResult<()> {
        async {
            let mut payload = Vec::new();
            entity_event.encode(&mut payload)?;
            // We are very close to the broker - so no need to pretend that we are at
            // risk of not receiving our messages. Right?
            let topic = self.generate_topic(entity_event)?;
            debug!(?topic, "topic");
            let msg = Message::new(self.generate_topic(entity_event)?, payload, 0);
            self.mqtt.publish(msg).compat().await?;
            Ok(())
        }
        .instrument(debug_span!("async_client_send", ?entity_event))
        .await
    }
}
