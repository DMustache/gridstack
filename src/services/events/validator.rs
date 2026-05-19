use crate::infrastructure::user_identifier::UserIdentifier;
use crate::services::events::entities::{
    EventClass, EventDefinitionRegistry, EventIntent, EventKind, MatrixEventContent,
    MessageEventKind, StateEventKind, StateKeyPolicy,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventIntentValidationError {
    InvalidEventType,
    InvalidEventContentShape { event_type: String },
    MissingStateKeyForStateEvent { event_type: String },
    MessageEventCannotHaveStateKey { event_type: String },
    StateKeyMustBeEmpty { event_type: String },
    InvalidMembershipStateKey { state_key: String },
}

pub fn validate_event_intent(event_intent: &EventIntent) -> Result<(), EventIntentValidationError> {
    let event_definition = EventDefinitionRegistry::for_event_kind(&event_intent.event_kind);

    if event_intent.event_type().trim().is_empty() {
        return Err(EventIntentValidationError::InvalidEventType);
    }
    match (
        event_definition.event_class,
        &event_intent.state_key,
        event_definition.state_key_policy,
    ) {
        (EventClass::State, Some(_), StateKeyPolicy::Required | StateKeyPolicy::MustBeEmpty)
        | (EventClass::Message, None, StateKeyPolicy::Forbidden) => {}
        (EventClass::State, None, _) => {
            return Err(EventIntentValidationError::MissingStateKeyForStateEvent {
                event_type: event_intent.event_type().to_owned(),
            });
        }
        (EventClass::Message, Some(_), _) => {
            return Err(EventIntentValidationError::MessageEventCannotHaveStateKey {
                event_type: event_intent.event_type().to_owned(),
            });
        }
        _ => {}
    }

    if matches!(
        event_definition.state_key_policy,
        StateKeyPolicy::MustBeEmpty
    ) && event_intent.state_key.as_deref() != Some("")
    {
        return Err(EventIntentValidationError::StateKeyMustBeEmpty {
            event_type: event_intent.event_type().to_owned(),
        });
    }

    if let EventKind::State(StateEventKind::RoomMember) = &event_intent.event_kind
        && event_intent
            .state_key
            .as_deref()
            .is_some_and(|state_key| UserIdentifier::try_from(state_key.to_owned()).is_err())
    {
        return Err(EventIntentValidationError::InvalidMembershipStateKey {
            state_key: event_intent.state_key.clone().unwrap_or_default(),
        });
    }

    validate_event_content(&event_intent.content)?;
    Ok(())
}

fn validate_event_content(content: &MatrixEventContent) -> Result<(), EventIntentValidationError> {
    match content {
        MatrixEventContent::RoomCreate(content) => {
            if content.creator.trim().is_empty() || content.room_version.trim().is_empty() {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: StateEventKind::RoomCreate.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::RoomMember(content) => match content.membership {
            crate::services::events::entities::MembershipContent::Join
            | crate::services::events::entities::MembershipContent::Invite
            | crate::services::events::entities::MembershipContent::Leave
            | crate::services::events::entities::MembershipContent::Ban
            | crate::services::events::entities::MembershipContent::Knock => {}
        },
        MatrixEventContent::RoomPowerLevels(_) => {}
        MatrixEventContent::RoomJoinRules { join_rule } => {
            if join_rule.trim().is_empty() {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: StateEventKind::RoomJoinRules.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::RoomHistoryVisibility { history_visibility } => {
            if history_visibility.trim().is_empty() {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: StateEventKind::RoomHistoryVisibility.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::RoomGuestAccess { guest_access } => {
            if guest_access.trim().is_empty() {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: StateEventKind::RoomGuestAccess.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::RoomCanonicalAlias { alias } => {
            if alias.trim().is_empty() {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: StateEventKind::RoomCanonicalAlias.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::RoomName { name } => {
            if name.trim().is_empty() {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: StateEventKind::RoomName.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::RoomTopic { topic } => {
            if topic.trim().is_empty() {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: StateEventKind::RoomTopic.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::RoomThirdPartyInvite {
            display_name,
            key_validity_url,
            public_key,
            medium,
            id_server,
        } => {
            if display_name.trim().is_empty()
                || key_validity_url.trim().is_empty()
                || public_key.trim().is_empty()
                || medium.trim().is_empty()
                || id_server.trim().is_empty()
            {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: StateEventKind::ThirdPartyInvite.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::RoomMessage { body, msgtype, .. } => {
            if body.trim().is_empty() || msgtype.trim().is_empty() {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: MessageEventKind::RoomMessage.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::RoomRedaction { redacts, .. } => {
            if redacts.trim().is_empty() {
                return Err(EventIntentValidationError::InvalidEventContentShape {
                    event_type: MessageEventKind::RoomRedaction.as_ref().to_owned(),
                });
            }
        }
        MatrixEventContent::CustomJson(_) => {}
    }

    Ok(())
}
