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

use std::rc::Rc;

use error_stack::Result;
use time::PrimitiveDateTime;

/// Errors that can occur when working with events.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EventsError {}

/// An event that can be published.
pub trait Event: EventInfo {
    /// The timestamp of the event.
    fn timestamp(&self) -> PrimitiveDateTime;

    /// The (unique) identifier of the event.
    fn identifier(&self) -> String;

    /// The class/type of the event. This is used to group/categorize events together.
    fn class(&self) -> String;
}

/// An Event sink that can publish events.
/// This could be a simple logger, a message queue, or a database.
pub trait EventSink {
    /// Publish an event.
    /// The parameters for this function are determined from the Event trait.
    fn publish_event(
        &self,
        data: serde_json::Value,
        identifier: String,
        topic: String,
        timestamp: PrimitiveDateTime,
    ) -> Result<(), EventsError>;
}

/// Hold the context information for any events
#[derive(Clone)]
pub struct EventContext {
    event_sink: Rc<Box<dyn EventSink>>,
    metadata: Vec<Rc<Box<dyn EventInfo>>>,
}

/// intermediary structure to build inplace events
pub struct EventBuilder {
    event_sink: Rc<Box<dyn EventSink>>,
    src_metadata: Vec<Rc<Box<dyn EventInfo>>>,
    event_metadata: Vec<Rc<Box<dyn EventInfo>>>,
    event: Box<dyn Event>,
}

impl EventBuilder {
    pub fn with<T: EventInfo + 'static>(mut self, info: T) -> Self {
        let boxed_event: Box<dyn EventInfo> = Box::new(info);
        self.event_metadata.push(boxed_event.into());
        self
    }
    pub fn emit(self) -> Result<(), EventsError> {
        self.event_sink.publish_event(
            self.data()?,
            self.event.identifier(),
            self.event.class(),
            self.event.timestamp(),
        )
    }
}

impl EventInfo for EventBuilder {
    fn data(&self) -> Result<serde_json::Value, EventsError> {
        self.src_metadata
            .iter()
            .chain(self.event_metadata.iter())
            .map(|info| info.data().map(|d| (info.key(), d)))
            .collect()
    }

    fn key(&self) -> String {
        self.event.key()
    }
}

impl EventContext {
    pub fn new(event_sink: Rc<Box<dyn EventSink>>) -> Self {
        Self {
            event_sink,
            metadata: Vec::new(),
        }
    }

    pub fn record_info<T: EventInfo + 'static>(&mut self, info: T) {
        let boxed_event: Box<dyn EventInfo> = Box::new(info);
        self.metadata.push(boxed_event.into());
    }

    pub fn emit(&self, event: Box<dyn Event>) -> Result<(), EventsError> {
        EventBuilder {
            event_sink: self.event_sink.clone(),
            src_metadata: self.metadata.clone(),
            event_metadata: vec![],
            event,
        }
        .emit()
    }

    pub fn event(&self, event: Box<dyn Event>) -> EventBuilder {
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
    fn data(&self) -> Result<serde_json::Value, EventsError>;

    /// The key identifying the data for an event.
    fn key(&self) -> String;
}
