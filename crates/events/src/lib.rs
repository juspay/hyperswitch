#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg_hide))]
#![cfg_attr(docsrs, doc(cfg_hide(doc)))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//!
//! A generic event handler system.
//! This library consists of 4 parts:
//! Event Sink: A trait that defines how events are published. This could be a simple logger, a message queue, or a database.
//! EventContext: A struct that holds the event sink and metadata about the event. This is used to create events. This can be used to add metadata to all events, such as the user who triggered the event.
//! EventInfo: A trait that defines the metadata that is sent with the event. It works with the EventContext to add metadata to all events.
//! Event: A trait that defines the event itself. This trait is used to define the data that is sent with the event and defines the event's type & identifier.
//!

use std::{collections::HashMap, sync::Arc};

use error_stack::{Result, ResultExt};
use masking::{ErasedMaskSerialize, Serialize};
use router_env::logger;
use serde::Serializer;
use serde_json::Value;
use time::PrimitiveDateTime;

/// Errors that can occur when working with events.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EventsError {
    /// An error occurred when publishing the event.
    #[error("Generic Error")]
    GenericError,
    /// An error occurred when serializing the event.
    #[error("Event serialization error")]
    SerializationError,
    /// An error occurred when publishing/producing the event.
    #[error("Event publishing error")]
    PublishError,
}

/// An event that can be published.
pub trait Event: EventInfo {
    /// The type of the event.
    type EventType;
    /// The timestamp of the event.
    fn timestamp(&self) -> PrimitiveDateTime;

    /// The (unique) identifier of the event.
    fn identifier(&self) -> String;

    /// The class/type of the event. This is used to group/categorize events together.
    fn class(&self) -> Self::EventType;
}

/// Hold the context information for any events
#[derive(Clone)]
pub struct EventContext<T, A>
where
    A: MessagingInterface<MessageClass = T>,
{
    message_sink: Arc<A>,
    metadata: HashMap<String, Value>,
}

/// intermediary structure to build events with in-place info.
#[must_use]
pub struct EventBuilder<T, A, E, D>
where
    A: MessagingInterface<MessageClass = T>,
    E: Event<EventType = T, Data = D>,
{
    message_sink: Arc<A>,
    metadata: HashMap<String, Value>,
    event: E,
}

struct RawEvent<T, A: Event<EventType = T>>(HashMap<String, Value>, A);

impl<T, A, E, D> EventBuilder<T, A, E, D>
where
    A: MessagingInterface<MessageClass = T>,
    E: Event<EventType = T, Data = D>,
{
    /// Add metadata to the event.
    pub fn with<F: ErasedMaskSerialize, G: EventInfo<Data = F> + 'static>(
        mut self,
        info: G,
    ) -> Self {
        info.data()
            .and_then(|i| {
                i.masked_serialize()
                    .change_context(EventsError::SerializationError)
            })
            .map_err(|e| {
                logger::error!("Error adding event info: {:?}", e);
            })
            .ok()
            .and_then(|data| self.metadata.insert(info.key(), data));
        self
    }
    /// Emit the event and log any errors.
    pub fn emit(self) {
        self.try_emit()
            .map_err(|e| {
                logger::error!("Error emitting event: {:?}", e);
            })
            .ok();
    }

    /// Emit the event.
    #[must_use = "make sure to call `emit` to actually emit the event"]
    pub fn try_emit(self) -> Result<(), EventsError> {
        let ts = self.event.timestamp();
        self.message_sink
            .send_message(RawEvent(self.metadata, self.event), ts)
    }
}

impl<T, A> Serialize for RawEvent<T, A>
where
    A: Event<EventType = T>,
{
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialize_map: HashMap<_, _> = self
            .0
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), v.masked_serialize().ok()?)))
            .collect();
        match self.1.data().map(|i| i.masked_serialize()) {
            Ok(Ok(Value::Object(map))) => {
                for (k, v) in map.into_iter() {
                    serialize_map.insert(k, v);
                }
            }
            Ok(Ok(i)) => {
                serialize_map.insert(self.1.key(), i);
            }
            i => {
                logger::error!("Error serializing event: {:?}", i);
            }
        };
        serialize_map.serialize(serializer)
    }
}

impl<T, A> EventContext<T, A>
where
    A: MessagingInterface<MessageClass = T>,
{
    /// Create a new event context.
    pub fn new(message_sink: A) -> Self {
        Self {
            message_sink: Arc::new(message_sink),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the event context.
    #[track_caller]
    pub fn record_info<G: ErasedMaskSerialize, E: EventInfo<Data = G> + 'static>(
        &mut self,
        info: E,
    ) {
        match info.data().and_then(|i| {
            i.masked_serialize()
                .change_context(EventsError::SerializationError)
        }) {
            Ok(data) => {
                self.metadata.insert(info.key(), data);
            }
            Err(e) => {
                logger::error!("Error recording event info: {:?}", e);
            }
        }
    }

    /// Emit an event.
    pub fn try_emit<E: Event<EventType = T>>(&self, event: E) -> Result<(), EventsError> {
        EventBuilder {
            message_sink: self.message_sink.clone(),
            metadata: self.metadata.clone(),
            event,
        }
        .try_emit()
    }

    /// Emit an event.
    pub fn emit<D, E: Event<EventType = T, Data = D>>(&self, event: E) {
        EventBuilder {
            message_sink: self.message_sink.clone(),
            metadata: self.metadata.clone(),
            event,
        }
        .emit()
    }

    /// Create an event builder.
    pub fn event<D, E: Event<EventType = T, Data = D>>(
        &self,
        event: E,
    ) -> EventBuilder<T, A, E, D> {
        EventBuilder {
            message_sink: self.message_sink.clone(),
            metadata: self.metadata.clone(),
            event,
        }
    }
}

/// Add information/metadata to the current context of an event.
pub trait EventInfo {
    /// The data that is sent with the event.
    type Data: ErasedMaskSerialize;
    /// The data that is sent with the event.
    fn data(&self) -> Result<Self::Data, EventsError>;

    /// The key identifying the data for an event.
    fn key(&self) -> String;
}

impl EventInfo for (String, String) {
    type Data = String;
    fn data(&self) -> Result<String, EventsError> {
        Ok(self.1.clone())
    }

    fn key(&self) -> String {
        self.0.clone()
    }
}

/// A messaging interface for sending messages/events.
/// This can be implemented for any messaging system, such as a message queue, a logger, or a database.
pub trait MessagingInterface {
    /// The type of the event used for categorization by the event publisher.
    type MessageClass;
    /// Send a message that follows the defined message class.
    fn send_message<T>(&self, data: T, timestamp: PrimitiveDateTime) -> Result<(), EventsError>
    where
        T: Message<Class = Self::MessageClass> + ErasedMaskSerialize;
}

/// A message that can be sent.
pub trait Message {
    /// The type of the event used for categorization by the event publisher.
    type Class;
    /// The type of the event used for categorization by the event publisher.
    fn get_message_class(&self) -> Self::Class;

    /// The (unique) identifier of the event.
    fn identifier(&self) -> String;
}

impl<T, A> Message for RawEvent<T, A>
where
    A: Event<EventType = T>,
{
    type Class = T;

    fn get_message_class(&self) -> Self::Class {
        self.1.class()
    }

    fn identifier(&self) -> String {
        self.1.identifier()
    }
}
