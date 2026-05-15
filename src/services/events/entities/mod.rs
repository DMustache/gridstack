use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{
    infrastructure::{server_name, user_identifier::UserIdentifier},
    services::rooms::entities::{RoomIdentifier, versions::RoomVersion},
};

pub mod event_kinds;

use event_kinds::RoomEventKind;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomEvent {
    pub content: Value,
    pub event_id: EventIdentifier,
    pub kind: RoomEventKind,
    pub origin_server_ts: u64,
    pub room_id: RoomIdentifier,
    pub sender: UserIdentifier,
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub unsigned: Option<Value>,
}

impl RoomEvent {
    pub fn validate(&self, room_version: RoomVersion) -> Result<(), RoomEventValidationError> {
        self.event_id.validate_for_version(room_version)?;
        self.room_id
            .validate_for_room_version(room_version.as_number())?;

        if self.kind == RoomEventKind::State && self.state_key.is_none() {
            return Err(RoomEventValidationError::MissingStateKeyForStateEvent {
                event_type: self.event_type.clone(),
            });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateEvent {
    RoomCreate(RoomCreateEvent),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCreateEvent {
    pub state_key: String,
    pub content: RoomCreateContent,
}

impl RoomCreateEvent {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        if !self.state_key.is_empty() {
            return Err(RoomEventValidationError::InvalidRoomCreateStateKey {
                received_state_key: self.state_key.clone(),
            });
        }

        self.content.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCreateContent {
    #[serde(default)]
    pub additional_creators: Option<Vec<UserIdentifier>>,
    pub creator: Option<UserIdentifier>,
    #[serde(rename = "m.federate", default = "default_true")]
    pub federate: bool,
    pub predecessor: Option<PreviousRoom>,
    #[serde(default)]
    pub room_version: RoomVersion,
    #[serde(rename = "type")]
    pub room_type: Option<String>,
}

impl RoomCreateContent {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        if self.room_version.supports_additional_creators() {
            if let Some(additional_creators) = self.additional_creators.as_ref()
                && additional_creators.is_empty()
            {
                return Err(RoomEventValidationError::AdditionalCreatorsCannotBeEmpty {
                    room_version: self.room_version,
                });
            }
        } else if self.additional_creators.is_some() {
            return Err(RoomEventValidationError::AdditionalCreatorsNotSupported {
                room_version: self.room_version,
            });
        }

        if self.room_version.requires_creator_field() && self.creator.is_none() {
            return Err(RoomEventValidationError::CreatorRequired {
                room_version: self.room_version,
            });
        }

        if !self.room_version.requires_creator_field() && self.creator.is_some() {
            return Err(RoomEventValidationError::CreatorNotSupported {
                room_version: self.room_version,
            });
        }

        if let Some(predecessor) = self.predecessor.as_ref() {
            predecessor.validate()?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventIdentifier(String);

impl EventIdentifier {
    pub fn parse(value: impl Into<String>) -> Option<Self> {
        let candidate = value.into();
        if !candidate.starts_with('$') || candidate.len() < 2 {
            return None;
        }

        Some(Self(candidate))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate_for_version(
        &self,
        room_version: RoomVersion,
    ) -> Result<(), RoomEventValidationError> {
        match room_version {
            RoomVersion::V1 | RoomVersion::V2 => {
                let suffix = self.0.strip_prefix('$').and_then(|identifier| {
                    server_name::split_opaque_identifier_and_server_name(identifier)
                });
                if suffix.is_none() {
                    return Err(RoomEventValidationError::EventIdentifierMissingDomain {
                        room_version,
                        event_identifier: self.0.clone(),
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviousRoom {
    pub event_id: EventIdentifier,
    pub room_id: RoomIdentifier,
}

impl PreviousRoom {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        self.room_id.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageEvent {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventHash {
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventReference(pub EventIdentifier, pub EventHash);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventUnsignedData {
    pub age: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomV1PersistentDataUnit {
    pub auth_events: Vec<EventReference>,
    pub content: Value,
    pub depth: u64,
    pub event_id: EventIdentifier,
    pub hashes: EventHash,
    pub origin_server_ts: u64,
    pub prev_events: Vec<EventReference>,
    pub redacts: Option<EventIdentifier>,
    pub room_id: RoomIdentifier,
    pub sender: UserIdentifier,
    pub signatures: BTreeMap<String, BTreeMap<String, String>>,
    pub state_key: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub unsigned: Option<EventUnsignedData>,
}

impl RoomV1PersistentDataUnit {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        const ROOM_V1_MAX_AUTH_EVENTS: usize = 10;
        const ROOM_V1_MAX_PREV_EVENTS: usize = 20;
        const ROOM_V1_MAX_DEPTH: u64 = i64::MAX as u64;

        self.event_id.validate_for_version(RoomVersion::V1)?;
        self.room_id
            .validate_for_room_version(RoomVersion::V1.as_number())?;

        if self.auth_events.len() > ROOM_V1_MAX_AUTH_EVENTS {
            return Err(RoomEventValidationError::TooManyAuthEvents {
                max_allowed: ROOM_V1_MAX_AUTH_EVENTS,
                actual: self.auth_events.len(),
            });
        }

        if self.prev_events.len() > ROOM_V1_MAX_PREV_EVENTS {
            return Err(RoomEventValidationError::TooManyPrevEvents {
                max_allowed: ROOM_V1_MAX_PREV_EVENTS,
                actual: self.prev_events.len(),
            });
        }

        if self.depth > ROOM_V1_MAX_DEPTH {
            return Err(RoomEventValidationError::DepthExceedsAllowedMaximum {
                max_allowed: ROOM_V1_MAX_DEPTH,
                actual: self.depth,
            });
        }

        if self.event_type.trim().is_empty() {
            return Err(RoomEventValidationError::EmptyEventType);
        }

        if self.hashes.sha256.trim().is_empty() {
            return Err(RoomEventValidationError::MissingEventHash { location: "hashes" });
        }

        if self.signatures.is_empty() {
            return Err(RoomEventValidationError::MissingSignatures);
        }

        Self::validate_event_references("auth_events", &self.auth_events)?;
        Self::validate_event_references("prev_events", &self.prev_events)?;

        if self.event_type == "m.room.create" {
            self.validate_create_event_rules(RoomVersion::V1)?;
        }

        if self.event_type == "m.room.aliases" {
            self.validate_room_v1_alias_event_rules()?;
        }

        Ok(())
    }

    fn validate_event_references(
        reference_list_name: &'static str,
        references: &[EventReference],
    ) -> Result<(), RoomEventValidationError> {
        for (reference_index, reference) in references.iter().enumerate() {
            reference.0.validate_for_version(RoomVersion::V1)?;
            if reference.1.sha256.trim().is_empty() {
                return Err(RoomEventValidationError::MissingEventHashInReference {
                    reference_list_name,
                    reference_index,
                });
            }
        }

        Ok(())
    }

    fn validate_create_event_rules(
        &self,
        expected_room_version: RoomVersion,
    ) -> Result<(), RoomEventValidationError> {
        if !self.prev_events.is_empty() {
            return Err(
                RoomEventValidationError::CreateEventCannotHavePreviousEvents {
                    actual: self.prev_events.len(),
                },
            );
        }

        let sender_server_name = self.sender.server_name().ok_or_else(|| {
            RoomEventValidationError::InvalidSenderIdentifier {
                sender: self.sender.as_str().to_owned(),
            }
        })?;
        let room_server_name = self.room_id.server_name().ok_or_else(|| {
            RoomEventValidationError::InvalidRoomIdentifierForCreateEvent {
                room_identifier: self.room_id.as_str().to_owned(),
            }
        })?;

        if sender_server_name != room_server_name {
            return Err(
                RoomEventValidationError::CreateEventRoomAndSenderDomainMismatch {
                    sender_domain: sender_server_name.to_owned(),
                    room_domain: room_server_name.to_owned(),
                },
            );
        }

        let create_content: RoomCreateContent = serde_json::from_value(self.content.clone())
            .map_err(
                |error| RoomEventValidationError::InvalidCreateEventContent {
                    reason: error.to_string(),
                },
            )?;

        if create_content.room_version != expected_room_version {
            return Err(RoomEventValidationError::CreateEventMustTargetRoomVersion {
                expected_room_version,
                room_version: create_content.room_version,
            });
        }
        create_content.validate()?;

        Ok(())
    }

    fn validate_room_v1_alias_event_rules(&self) -> Result<(), RoomEventValidationError> {
        let state_key = self
            .state_key
            .as_ref()
            .ok_or(RoomEventValidationError::AliasEventRequiresStateKey)?;
        let sender_server_name = self.sender.server_name().ok_or_else(|| {
            RoomEventValidationError::InvalidSenderIdentifier {
                sender: self.sender.as_str().to_owned(),
            }
        })?;

        if state_key != sender_server_name {
            return Err(
                RoomEventValidationError::AliasEventStateKeyMustMatchSenderDomain {
                    state_key: state_key.to_owned(),
                    sender_domain: sender_server_name.to_owned(),
                },
            );
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomV2PersistentDataUnit {
    pub inner: RoomV1PersistentDataUnit,
}

impl RoomV2PersistentDataUnit {
    pub fn validate(&self) -> Result<(), RoomEventValidationError> {
        const ROOM_V2_MAX_AUTH_EVENTS: usize = 10;
        const ROOM_V2_MAX_PREV_EVENTS: usize = 20;
        const ROOM_V2_MAX_DEPTH: u64 = i64::MAX as u64;

        self.inner.event_id.validate_for_version(RoomVersion::V2)?;
        self.inner
            .room_id
            .validate_for_room_version(RoomVersion::V2.as_number())?;

        if self.inner.auth_events.len() > ROOM_V2_MAX_AUTH_EVENTS {
            return Err(RoomEventValidationError::TooManyAuthEvents {
                max_allowed: ROOM_V2_MAX_AUTH_EVENTS,
                actual: self.inner.auth_events.len(),
            });
        }

        if self.inner.prev_events.len() > ROOM_V2_MAX_PREV_EVENTS {
            return Err(RoomEventValidationError::TooManyPrevEvents {
                max_allowed: ROOM_V2_MAX_PREV_EVENTS,
                actual: self.inner.prev_events.len(),
            });
        }

        if self.inner.depth > ROOM_V2_MAX_DEPTH {
            return Err(RoomEventValidationError::DepthExceedsAllowedMaximum {
                max_allowed: ROOM_V2_MAX_DEPTH,
                actual: self.inner.depth,
            });
        }

        if self.inner.event_type.trim().is_empty() {
            return Err(RoomEventValidationError::EmptyEventType);
        }

        if self.inner.hashes.sha256.trim().is_empty() {
            return Err(RoomEventValidationError::MissingEventHash { location: "hashes" });
        }

        if self.inner.signatures.is_empty() {
            return Err(RoomEventValidationError::MissingSignatures);
        }

        RoomV1PersistentDataUnit::validate_event_references(
            "auth_events",
            &self.inner.auth_events,
        )?;
        RoomV1PersistentDataUnit::validate_event_references(
            "prev_events",
            &self.inner.prev_events,
        )?;

        if self.inner.event_type == "m.room.create" {
            self.inner.validate_create_event_rules(RoomVersion::V2)?;
        }

        if self.inner.event_type == "m.room.aliases" {
            self.inner.validate_room_v1_alias_event_rules()?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StateEventTypeAndKey {
    pub event_type: String,
    pub state_key: String,
}

pub type RoomStateSnapshot = BTreeMap<StateEventTypeAndKey, EventIdentifier>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomV2StateEvent {
    pub event_id: EventIdentifier,
    pub event_type: String,
    pub state_key: String,
    pub sender: UserIdentifier,
    pub origin_server_ts: u64,
    pub auth_events: Vec<EventIdentifier>,
    pub membership: Option<String>,
    pub rejected_for_auth_chain: bool,
    pub sender_power_level: i64,
}

impl RoomV2StateEvent {
    pub fn state_key_tuple(&self) -> StateEventTypeAndKey {
        StateEventTypeAndKey {
            event_type: self.event_type.clone(),
            state_key: self.state_key.clone(),
        }
    }

    pub fn is_power_event(&self) -> bool {
        if self.event_type == "m.room.power_levels" || self.event_type == "m.room.join_rules" {
            return true;
        }

        self.event_type == "m.room.member"
            && (self.membership.as_deref() == Some("leave")
                || self.membership.as_deref() == Some("ban"))
            && self.sender.as_str() != self.state_key
    }
}

pub trait RoomV2AuthorizationEvaluator {
    fn is_event_allowed(&self, event: &RoomV2StateEvent, current_state: &RoomStateSnapshot)
    -> bool;
}

pub struct AllowAllRoomV2AuthorizationEvaluator;

impl RoomV2AuthorizationEvaluator for AllowAllRoomV2AuthorizationEvaluator {
    fn is_event_allowed(
        &self,
        _event: &RoomV2StateEvent,
        _current_state: &RoomStateSnapshot,
    ) -> bool {
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomV2StateResolutionInput {
    pub state_sets: Vec<RoomStateSnapshot>,
    pub events: BTreeMap<EventIdentifier, RoomV2StateEvent>,
}

pub fn resolve_room_state_v2<E: RoomV2AuthorizationEvaluator>(
    input: &RoomV2StateResolutionInput,
    evaluator: &E,
) -> Result<RoomStateSnapshot, RoomEventValidationError> {
    let (unconflicted_state, conflicted_state_set) =
        partition_unconflicted_and_conflicted_state_sets(input);
    let auth_difference = calculate_auth_difference(input)?;

    let full_conflicted_set: BTreeSet<EventIdentifier> = conflicted_state_set
        .union(&auth_difference)
        .cloned()
        .collect();

    let mut power_events_and_auth_chains: BTreeSet<EventIdentifier> = full_conflicted_set
        .iter()
        .filter_map(|event_id| {
            let event = lookup_v2_state_event(input, event_id).ok()?;
            event.is_power_event().then_some(event_id.clone())
        })
        .collect();
    let power_events = power_events_and_auth_chains.clone();
    for power_event_id in power_events {
        for auth_event_id in collect_auth_chain(input, &power_event_id)? {
            if full_conflicted_set.contains(&auth_event_id) {
                power_events_and_auth_chains.insert(auth_event_id);
            }
        }
    }

    let step_one_event_ids = reverse_topological_power_order(input, &power_events_and_auth_chains)?;
    let mut partially_resolved_state =
        iterative_auth_checks(input, evaluator, &unconflicted_state, &step_one_event_ids)?;

    let remaining_event_ids: BTreeSet<EventIdentifier> = full_conflicted_set
        .difference(&power_events_and_auth_chains)
        .cloned()
        .collect();
    let step_two_event_ids =
        order_by_mainline(input, &remaining_event_ids, &partially_resolved_state)?;

    partially_resolved_state = iterative_auth_checks(
        input,
        evaluator,
        &partially_resolved_state,
        &step_two_event_ids,
    )?;

    for (state_key, event_id) in unconflicted_state {
        partially_resolved_state.insert(state_key, event_id);
    }

    Ok(partially_resolved_state)
}

fn partition_unconflicted_and_conflicted_state_sets(
    input: &RoomV2StateResolutionInput,
) -> (RoomStateSnapshot, BTreeSet<EventIdentifier>) {
    let mut all_state_keys = BTreeSet::new();
    for state_set in &input.state_sets {
        all_state_keys.extend(state_set.keys().cloned());
    }

    let mut unconflicted_state = RoomStateSnapshot::new();
    let mut conflicted_state_set = BTreeSet::new();

    for state_key in all_state_keys {
        let mut values = BTreeSet::new();
        let mut is_unconflicted = true;

        for state_set in &input.state_sets {
            match state_set.get(&state_key) {
                Some(event_id) => {
                    values.insert(event_id.clone());
                }
                None => {
                    is_unconflicted = false;
                }
            }
        }

        if is_unconflicted && values.len() == 1 {
            if let Some(event_id) = values.iter().next().cloned() {
                unconflicted_state.insert(state_key, event_id);
            }
            continue;
        }

        conflicted_state_set.extend(values);
    }

    (unconflicted_state, conflicted_state_set)
}

fn calculate_auth_difference(
    input: &RoomV2StateResolutionInput,
) -> Result<BTreeSet<EventIdentifier>, RoomEventValidationError> {
    if input.state_sets.is_empty() {
        return Ok(BTreeSet::new());
    }

    let mut full_auth_chains = Vec::with_capacity(input.state_sets.len());
    for state_set in &input.state_sets {
        let mut auth_chain = BTreeSet::new();
        for event_id in state_set.values() {
            auth_chain.extend(collect_auth_chain(input, event_id)?);
        }
        full_auth_chains.push(auth_chain);
    }

    let union = full_auth_chains
        .iter()
        .flat_map(|chain| chain.iter().cloned())
        .collect::<BTreeSet<_>>();
    let intersection =
        full_auth_chains
            .iter()
            .skip(1)
            .fold(full_auth_chains[0].clone(), |mut acc, chain| {
                acc.retain(|event_id| chain.contains(event_id));
                acc
            });

    Ok(union
        .difference(&intersection)
        .cloned()
        .collect::<BTreeSet<_>>())
}

fn collect_auth_chain(
    input: &RoomV2StateResolutionInput,
    root_event_id: &EventIdentifier,
) -> Result<BTreeSet<EventIdentifier>, RoomEventValidationError> {
    let mut visited = BTreeSet::new();
    let mut stack = Vec::new();

    let root_event = lookup_v2_state_event(input, root_event_id)?;
    stack.extend(root_event.auth_events.iter().cloned());

    while let Some(event_id) = stack.pop() {
        if !visited.insert(event_id.clone()) {
            continue;
        }

        let event = lookup_v2_state_event(input, &event_id)?;
        stack.extend(event.auth_events.iter().cloned());
    }

    Ok(visited)
}

fn reverse_topological_power_order(
    input: &RoomV2StateResolutionInput,
    event_ids: &BTreeSet<EventIdentifier>,
) -> Result<Vec<EventIdentifier>, RoomEventValidationError> {
    if event_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut indegree = BTreeMap::<EventIdentifier, usize>::new();
    let mut adjacency = BTreeMap::<EventIdentifier, Vec<EventIdentifier>>::new();
    for event_id in event_ids {
        indegree.insert(event_id.clone(), 0);
        adjacency.insert(event_id.clone(), Vec::new());
    }

    for event_id in event_ids {
        let event = lookup_v2_state_event(input, event_id)?;
        for auth_event_id in &event.auth_events {
            if !event_ids.contains(auth_event_id) {
                continue;
            }

            *indegree.entry(event_id.clone()).or_default() += 1;
            adjacency
                .entry(auth_event_id.clone())
                .or_default()
                .push(event_id.clone());
        }
    }

    let mut candidates = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(event_id, _)| event_id.clone())
        .collect::<Vec<_>>();
    let mut ordered = Vec::with_capacity(event_ids.len());

    while !candidates.is_empty() {
        candidates.sort_by(|left, right| compare_power_order_events(input, left, right));
        let next_event_id = candidates.remove(0);
        ordered.push(next_event_id.clone());

        if let Some(dependents) = adjacency.get(&next_event_id) {
            for dependent_event_id in dependents {
                let entry = indegree.entry(dependent_event_id.clone()).or_default();
                *entry = entry.saturating_sub(1);
                if *entry == 0 {
                    candidates.push(dependent_event_id.clone());
                }
            }
        }
    }

    if ordered.len() < event_ids.len() {
        let mut unresolved = event_ids
            .iter()
            .filter(|event_id| !ordered.contains(event_id))
            .cloned()
            .collect::<Vec<_>>();
        unresolved.sort_by(|left, right| compare_power_order_events(input, left, right));
        ordered.extend(unresolved);
    }

    Ok(ordered)
}

fn compare_power_order_events(
    input: &RoomV2StateResolutionInput,
    left: &EventIdentifier,
    right: &EventIdentifier,
) -> std::cmp::Ordering {
    let left_event = input.events.get(left);
    let right_event = input.events.get(right);

    match (left_event, right_event) {
        (Some(left_event), Some(right_event)) => right_event
            .sender_power_level
            .cmp(&left_event.sender_power_level)
            .then_with(|| {
                left_event
                    .origin_server_ts
                    .cmp(&right_event.origin_server_ts)
            })
            .then_with(|| {
                left_event
                    .event_id
                    .as_str()
                    .cmp(right_event.event_id.as_str())
            }),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.as_str().cmp(right.as_str()),
    }
}

fn iterative_auth_checks<E: RoomV2AuthorizationEvaluator>(
    input: &RoomV2StateResolutionInput,
    evaluator: &E,
    initial_state: &RoomStateSnapshot,
    ordered_event_ids: &[EventIdentifier],
) -> Result<RoomStateSnapshot, RoomEventValidationError> {
    let mut resolved_state = initial_state.clone();
    for event_id in ordered_event_ids {
        let event = lookup_v2_state_event(input, event_id)?;
        if event.rejected_for_auth_chain {
            continue;
        }

        if evaluator.is_event_allowed(event, &resolved_state) {
            resolved_state.insert(event.state_key_tuple(), event.event_id.clone());
        }
    }

    Ok(resolved_state)
}

fn order_by_mainline(
    input: &RoomV2StateResolutionInput,
    event_ids: &BTreeSet<EventIdentifier>,
    partially_resolved_state: &RoomStateSnapshot,
) -> Result<Vec<EventIdentifier>, RoomEventValidationError> {
    let mut sorted = event_ids.iter().cloned().collect::<Vec<_>>();
    if sorted.is_empty() {
        return Ok(sorted);
    }

    let power_level_key = StateEventTypeAndKey {
        event_type: "m.room.power_levels".to_owned(),
        state_key: String::new(),
    };
    let power_event_id = partially_resolved_state.get(&power_level_key).cloned();
    let mainline_positions = build_mainline_positions(input, power_event_id.as_ref())?;

    sorted.sort_by(|left, right| {
        let left_event = input.events.get(left);
        let right_event = input.events.get(right);

        match (left_event, right_event) {
            (Some(left_event), Some(right_event)) => {
                let left_position =
                    closest_mainline_position(input, left_event, &mainline_positions);
                let right_position =
                    closest_mainline_position(input, right_event, &mainline_positions);

                left_position
                    .cmp(&right_position)
                    .then_with(|| {
                        left_event
                            .origin_server_ts
                            .cmp(&right_event.origin_server_ts)
                    })
                    .then_with(|| {
                        left_event
                            .event_id
                            .as_str()
                            .cmp(right_event.event_id.as_str())
                    })
            }
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => left.as_str().cmp(right.as_str()),
        }
    });

    Ok(sorted)
}

fn build_mainline_positions(
    input: &RoomV2StateResolutionInput,
    power_event_id: Option<&EventIdentifier>,
) -> Result<BTreeMap<EventIdentifier, i64>, RoomEventValidationError> {
    let Some(mut current_event_id) = power_event_id.cloned() else {
        return Ok(BTreeMap::new());
    };

    let mut mainline = Vec::new();
    loop {
        mainline.push(current_event_id.clone());
        let current_event = lookup_v2_state_event(input, &current_event_id)?;
        let Some(next_power_event_id) =
            select_power_level_auth_event_id(input, &current_event.auth_events)
        else {
            break;
        };
        current_event_id = next_power_event_id;
    }

    mainline.reverse();
    let mut positions = BTreeMap::new();
    for (index, event_id) in mainline.into_iter().enumerate() {
        let index_as_i64 = i64::try_from(index).unwrap_or(i64::MAX);
        positions.insert(event_id, index_as_i64);
    }

    Ok(positions)
}

fn closest_mainline_position(
    input: &RoomV2StateResolutionInput,
    event: &RoomV2StateEvent,
    mainline_positions: &BTreeMap<EventIdentifier, i64>,
) -> i64 {
    let mut cursor = select_power_level_auth_event_id(input, &event.auth_events);
    while let Some(power_event_id) = cursor {
        if let Some(position) = mainline_positions.get(&power_event_id) {
            return *position;
        }

        let Some(next_event) = input.events.get(&power_event_id) else {
            break;
        };
        cursor = select_power_level_auth_event_id(input, &next_event.auth_events);
    }

    -1
}

fn select_power_level_auth_event_id(
    input: &RoomV2StateResolutionInput,
    auth_event_ids: &[EventIdentifier],
) -> Option<EventIdentifier> {
    auth_event_ids
        .iter()
        .filter(|event_id| {
            input
                .events
                .get(*event_id)
                .is_some_and(|event| event.event_type == "m.room.power_levels")
        })
        .cloned()
        .min_by(|left, right| left.as_str().cmp(right.as_str()))
}

fn lookup_v2_state_event<'a>(
    input: &'a RoomV2StateResolutionInput,
    event_id: &EventIdentifier,
) -> Result<&'a RoomV2StateEvent, RoomEventValidationError> {
    input.events.get(event_id).ok_or_else(|| {
        RoomEventValidationError::MissingRoomV2ResolutionEvent {
            event_identifier: event_id.as_str().to_owned(),
        }
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RoomEventValidationError {
    #[error("state event `{event_type}` must include a state_key")]
    MissingStateKeyForStateEvent { event_type: String },
    #[error("m.room.create state_key must be empty, got `{received_state_key}`")]
    InvalidRoomCreateStateKey { received_state_key: String },
    #[error("room version `{room_version}` requires `creator` in m.room.create content")]
    CreatorRequired { room_version: RoomVersion },
    #[error("room version `{room_version}` does not support `creator` in m.room.create content")]
    CreatorNotSupported { room_version: RoomVersion },
    #[error(
        "room version `{room_version}` does not support `additional_creators` in m.room.create content"
    )]
    AdditionalCreatorsNotSupported { room_version: RoomVersion },
    #[error("room version `{room_version}` requires non-empty `additional_creators` when provided")]
    AdditionalCreatorsCannotBeEmpty { room_version: RoomVersion },
    #[error(
        "event identifier `{event_identifier}` in room version `{room_version}` must include a valid server-name domain"
    )]
    EventIdentifierMissingDomain {
        room_version: RoomVersion,
        event_identifier: String,
    },
    #[error(
        "room version 1 allows at most `{max_allowed}` auth events, received `{actual}` auth events"
    )]
    TooManyAuthEvents { max_allowed: usize, actual: usize },
    #[error(
        "room version 1 allows at most `{max_allowed}` previous events, received `{actual}` previous events"
    )]
    TooManyPrevEvents { max_allowed: usize, actual: usize },
    #[error("room version 1 requires depth <= `{max_allowed}` (2^63 - 1), received `{actual}`")]
    DepthExceedsAllowedMaximum { max_allowed: u64, actual: u64 },
    #[error("event type cannot be empty")]
    EmptyEventType,
    #[error("missing or empty sha256 hash in `{location}`")]
    MissingEventHash { location: &'static str },
    #[error("missing or empty sha256 hash in `{reference_list_name}` at index `{reference_index}`")]
    MissingEventHashInReference {
        reference_list_name: &'static str,
        reference_index: usize,
    },
    #[error("event signatures cannot be empty")]
    MissingSignatures,
    #[error("room version 1 create event cannot have prev_events, received `{actual}`")]
    CreateEventCannotHavePreviousEvents { actual: usize },
    #[error("invalid sender user id `{sender}`")]
    InvalidSenderIdentifier { sender: String },
    #[error("invalid room id `{room_identifier}` for m.room.create in room version 1")]
    InvalidRoomIdentifierForCreateEvent { room_identifier: String },
    #[error(
        "room version 1 create event requires sender and room domains to match, got sender `{sender_domain}` and room `{room_domain}`"
    )]
    CreateEventRoomAndSenderDomainMismatch {
        sender_domain: String,
        room_domain: String,
    },
    #[error("invalid m.room.create content: {reason}")]
    InvalidCreateEventContent { reason: String },
    #[error(
        "create event content must target room version `{expected_room_version}`, received `{room_version}`"
    )]
    CreateEventMustTargetRoomVersion {
        expected_room_version: RoomVersion,
        room_version: RoomVersion,
    },
    #[error("room version 1 alias events require state_key")]
    AliasEventRequiresStateKey,
    #[error(
        "room version 1 alias event state_key `{state_key}` must match sender domain `{sender_domain}`"
    )]
    AliasEventStateKeyMustMatchSenderDomain {
        state_key: String,
        sender_domain: String,
    },
    #[error(
        "state resolution input is missing event `{event_identifier}` referenced by state/auth edges"
    )]
    MissingRoomV2ResolutionEvent { event_identifier: String },
    #[error(transparent)]
    InvalidRoomIdentifier(#[from] crate::services::rooms::entities::RoomIdentifierValidationError),
}

const fn default_true() -> bool {
    true
}
