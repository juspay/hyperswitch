#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg_hide))]
#![cfg_attr(docsrs, doc(cfg_hide(doc)))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//!
//! A generic event handler system.
//! This library consists of 4 parts:
//! Event Sink: A trait that defines how events are published. This could be a simple logger, a message queue, or a database.
//! EventContext: A struct that holds the event sink and metadata about the event. This is used to create events. This can be used to add metadata to all events, such as the user who triggered the event.
//! EventInfo: A trait that defines the metadata that is sent with the event. This trait is used to define the data that is sent with the event it works with the EventContext to add metadata to all events.
//! Event: A trait that defines the event itself. This trait is used to define the data that is sent with the event and defines the event's type & identifier.
//!

use std::{collections::HashMap, rc::Rc};

use error_stack::Result;
use time::OffsetDateTime;

/// Errors that can occur when working with events.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EventsError {
    /// An error occurred when publishing the event.
    #[error("Generic Error")]
    GenericError,
    /// An error occurred when serializing the event.
    #[error("Event serialization error: {0}")]
    SerializationError(String),
}

/// An event that can be published.
pub trait Event: EventInfo {
    /// The type of the event.
    type EventType;
    /// The timestamp of the event.
    fn timestamp(&self) -> OffsetDateTime;

    /// The (unique) identifier of the event.
    fn identifier(&self) -> String;

    /// The class/type of the event. This is used to group/categorize events together.
    fn class(&self) -> Self::EventType;
}

/// An Event sink that can publish events.
/// This could be a simple logger, a message queue, or a database.
pub trait EventSink<T> {
    /// Publish an event.
    /// The parameters for this function are determined from the Event trait.
    fn publish_event(
        &self,
        data: serde_json::Value,
        identifier: String,
        topic: T,
        timestamp: OffsetDateTime,
    ) -> Result<(), EventsError>;
}

/// Hold the context information for any events
#[derive(Clone)]
pub struct EventContext<T> {
    event_sink: Rc<Box<dyn EventSink<T>>>,
    metadata: Vec<Rc<Box<dyn EventInfo>>>,
}

/// intermediary structure to build events with in-place info.
pub struct EventBuilder<T> {
    event_sink: Rc<Box<dyn EventSink<T>>>,
    src_metadata: Vec<Rc<Box<dyn EventInfo>>>,
    event_metadata: Vec<Rc<Box<dyn EventInfo>>>,
    event: Box<dyn Event<EventType = T>>,
}

impl<T> EventBuilder<T> {
    /// Add metadata to the event.
    pub fn with<E: EventInfo + 'static>(mut self, info: E) -> Self {
        let boxed_event: Box<dyn EventInfo> = Box::new(info);
        self.event_metadata.push(boxed_event.into());
        self
    }
    /// Emit the event.
    pub fn emit(self) -> Result<(), EventsError> {
        self.event_sink.publish_event(
            serde_json::Value::Object(self.data()?.into_iter().collect()),
            self.event.identifier(),
            self.event.class(),
            self.event.timestamp(),
        )
    }
}

impl<T> EventInfo for EventBuilder<T> {
    fn data(&self) -> Result<HashMap<String, serde_json::Value>, EventsError> {
        self.event_metadata
            .iter()
            .chain(self.src_metadata.iter())
            .rev()
            .map(|info| {
                info.data().map(|d| {
                    (
                        info.key(),
                        serde_json::Value::Object(d.into_iter().collect()),
                    )
                })
            })
            .chain(self.event.data()?.into_iter().map(Ok))
            .collect()
    }

    fn key(&self) -> String {
        self.event.key()
    }
}

impl<T> EventContext<T> {
    /// Create a new event context.
    pub fn new(event_sink: Rc<Box<dyn EventSink<T>>>) -> Self {
        Self {
            event_sink,
            metadata: Vec::new(),
        }
    }

    /// Add metadata to the event context.
    pub fn record_info<E: EventInfo + 'static>(&mut self, info: E) {
        let boxed_event: Box<dyn EventInfo> = Box::new(info);
        self.metadata.push(boxed_event.into());
    }

    /// Emit an event.
    pub fn emit(&self, event: Box<dyn Event<EventType = T>>) -> Result<(), EventsError> {
        EventBuilder {
            event_sink: self.event_sink.clone(),
            src_metadata: self.metadata.clone(),
            event_metadata: vec![],
            event,
        }
        .emit()
    }

    /// Create an event builder.
    pub fn event(&self, event: Box<dyn Event<EventType = T>>) -> EventBuilder<T> {
        EventBuilder {
            event_sink: self.event_sink.clone(),
            src_metadata: self.metadata.clone(),
            event_metadata: vec![],
            event,
        }
    }
}

/// Add information/metadata to the current context of an event.
pub trait EventInfo {
    /// The data that is sent with the event.
    fn data(&self) -> Result<HashMap<String, serde_json::Value>, EventsError>;

    /// The key identifying the data for an event.
    fn key(&self) -> String;
}
