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

use std::sync::Arc;

use error_stack::Result;
use masking::Serialize;
use serde::{ser::SerializeMap, Serializer};
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
    metadata: Vec<Arc<Box<dyn EventInfo>>>,
}

/// intermediary structure to build events with in-place info.
pub struct EventBuilder<T, A>
where
    A: MessagingInterface<MessageClass = T>,
{
    message_sink: Arc<A>,
    src_metadata: Vec<Arc<Box<dyn EventInfo>>>,
    event_metadata: Vec<Arc<Box<dyn EventInfo>>>,
    event: Box<dyn Event<EventType = T>>,
}

struct RawEvent<T>(Vec<Arc<Box<dyn EventInfo>>>, Box<dyn Event<EventType = T>>);

impl<T, A> EventBuilder<T, A>
where
    A: MessagingInterface<MessageClass = T>,
{
    /// Add metadata to the event.
    pub fn with<E: EventInfo + 'static>(mut self, info: E) -> Self {
        let boxed_event: Box<dyn EventInfo> = Box::new(info);
        self.event_metadata.push(boxed_event.into());
        self
    }
    /// Emit the event and log any errors.
    #[track_caller]
    pub fn emit(self) {
        self.try_emit()
            .map_err(|e| {
                router_env::logger::error!("Error emitting event: {:?}", e);
            })
            .ok();
    }

    /// Emit the event.
    #[must_use = "make sure to actually emit the event"]
    #[track_caller]
    pub fn try_emit(mut self) -> Result<(), EventsError> {
        self.event_metadata.append(&mut self.src_metadata);
        let ts = self.event.timestamp();
        self.message_sink
            .send_message(RawEvent(self.event_metadata.clone(), self.event), ts)
    }
}

impl<T, A> EventInfo for EventBuilder<T, A>
where
    A: MessagingInterface<MessageClass = T>,
{
    fn data(&self) -> Result<Value, EventsError> {
        let mut event_data = match self.event.data()? {
            Value::Object(map) => map,
            d => {
                let mut map = serde_json::Map::new();
                map.insert(self.event.key(), d);
                map
            }
        };
        let mut data: serde_json::Map<String, Value> = self
            .src_metadata
            .iter()
            .chain(self.event_metadata.iter())
            .map(|info| info.data().map(|d| (info.key(), d)))
            .collect::<Result<serde_json::Map<_, _>, _>>()?;
        data.append(&mut event_data);
        Ok(Value::Object(data))
    }

    fn key(&self) -> String {
        self.event.key()
    }
}

impl<T> Serialize for RawEvent<T> {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialize_map = serializer.serialize_map(None)?;
        self.0
            .iter()
            .map(|info| Some((info.key(), info.data().ok()?)))
            .filter_map(|i| i)
            .for_each(|(k, v)| {
                serialize_map.serialize_entry(&k, &v).ok();
            });
        match self.1.data() {
            Ok(Value::Object(map)) => {
                for (k, v) in map.into_iter() {
                    serialize_map.serialize_entry(&k, &v)?;
                }
            }
            Ok(i) => serialize_map.serialize_entry(&self.1.key(), &i)?,
            _ => {}
        };
        serialize_map.end()
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
            metadata: Vec::new(),
        }
    }

    /// Add metadata to the event context.
    pub fn record_info<E: EventInfo + 'static>(&mut self, info: E) {
        let boxed_event: Box<dyn EventInfo> = Box::new(info);
        self.metadata.push(boxed_event.into());
    }

    /// Emit an event.
    #[must_use = "make sure to actually emit the event"]
    #[track_caller]
    pub fn try_emit(&self, event: Box<dyn Event<EventType = T>>) -> Result<(), EventsError> {
        EventBuilder {
            message_sink: self.message_sink.clone(),
            src_metadata: self.metadata.clone(),
            event_metadata: vec![],
            event,
        }
        .try_emit()
    }

    /// Emit an event.
    /// This silences the error thrown when emitting an event.
    #[track_caller]
    pub fn emit(&self, event: Box<dyn Event<EventType = T>>) {
        EventBuilder {
            message_sink: self.message_sink.clone(),
            src_metadata: self.metadata.clone(),
            event_metadata: vec![],
            event,
        }
        .emit()
    }

    /// Create an event builder.
    pub fn event(&self, event: Box<dyn Event<EventType = T>>) -> EventBuilder<T, A> {
        EventBuilder {
            message_sink: self.message_sink.clone(),
            src_metadata: self.metadata.clone(),
            event_metadata: vec![],
            event,
        }
    }
}

/// Add information/metadata to the current context of an event.
pub trait EventInfo {
    /// The data that is sent with the event.
    fn data(&self) -> Result<Value, EventsError>;

    /// The key identifying the data for an event.
    fn key(&self) -> String;
}

impl EventInfo for (String, String) {
    fn data(&self) -> Result<Value, EventsError> {
        Ok(Value::String(self.1.clone()))
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
        T: Message<Class = Self::MessageClass> + Serialize;
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

impl<T> Message for RawEvent<T> {
    type Class = T;

    fn get_message_class(&self) -> Self::Class {
        self.1.class()
    }

    fn identifier(&self) -> String {
        self.1.identifier()
    }
}
