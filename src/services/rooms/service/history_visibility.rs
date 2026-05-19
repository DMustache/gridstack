pub struct EventVisibilityEvaluationInput<'a> {
    pub event_type: &'a str,
    pub event_state_key: Option<&'a str>,
    pub requesting_user_id: &'a str,
    pub history_visibility_at_event: Option<&'a str>,
    pub history_visibility_before_event: Option<&'a str>,
    pub history_visibility_after_event: Option<&'a str>,
    pub membership_at_event: Option<&'a str>,
    pub membership_before_event: Option<&'a str>,
    pub membership_after_event: Option<&'a str>,
    pub user_joined_since_event: bool,
}

#[derive(Clone, Copy, Debug)]
enum RoomHistoryVisibility {
    WorldReadable,
    Shared,
    Invited,
    Joined,
}

fn parse_room_history_visibility(value: Option<&str>) -> RoomHistoryVisibility {
    match value {
        Some("world_readable") => RoomHistoryVisibility::WorldReadable,
        Some("invited") => RoomHistoryVisibility::Invited,
        Some("joined") => RoomHistoryVisibility::Joined,
        Some("shared") => RoomHistoryVisibility::Shared,
        _ => RoomHistoryVisibility::Shared,
    }
}

fn evaluate_general_event_visibility(
    history_visibility: RoomHistoryVisibility,
    membership: Option<&str>,
    user_joined_since_event: bool,
) -> bool {
    if matches!(history_visibility, RoomHistoryVisibility::WorldReadable) {
        return true;
    }
    if membership == Some("join") {
        return true;
    }
    if matches!(history_visibility, RoomHistoryVisibility::Shared) && user_joined_since_event {
        return true;
    }
    membership == Some("invite") && matches!(history_visibility, RoomHistoryVisibility::Invited)
}

pub fn evaluate_event_visibility(input: &EventVisibilityEvaluationInput<'_>) -> bool {
    let history_visibility_at_event =
        parse_room_history_visibility(input.history_visibility_at_event);
    let mut visible = evaluate_general_event_visibility(
        history_visibility_at_event,
        input.membership_at_event,
        input.user_joined_since_event,
    );

    if input.event_type == "m.room.history_visibility" && input.event_state_key == Some("") {
        let visible_before = evaluate_general_event_visibility(
            parse_room_history_visibility(input.history_visibility_before_event),
            input.membership_at_event,
            input.user_joined_since_event,
        );
        let visible_after = evaluate_general_event_visibility(
            parse_room_history_visibility(
                input
                    .history_visibility_after_event
                    .or(input.history_visibility_at_event),
            ),
            input.membership_at_event,
            input.user_joined_since_event,
        );
        visible = visible_before || visible_after;
    }

    if input.event_type == "m.room.member"
        && input.event_state_key == Some(input.requesting_user_id)
    {
        let visible_before = evaluate_general_event_visibility(
            history_visibility_at_event,
            input.membership_before_event,
            input.user_joined_since_event,
        );
        let visible_after = evaluate_general_event_visibility(
            history_visibility_at_event,
            input.membership_after_event.or(input.membership_at_event),
            input.user_joined_since_event,
        );
        visible = visible || visible_before || visible_after;
    }

    visible
}

#[cfg(test)]
mod tests {
    use super::{EventVisibilityEvaluationInput, evaluate_event_visibility};

    #[test]
    fn history_visibility_event_visible_when_previous_policy_allows() {
        let input = EventVisibilityEvaluationInput {
            event_type: "m.room.history_visibility",
            event_state_key: Some(""),
            requesting_user_id: "@alice:example.org",
            history_visibility_at_event: Some("joined"),
            history_visibility_before_event: Some("world_readable"),
            history_visibility_after_event: Some("joined"),
            membership_at_event: Some("leave"),
            membership_before_event: None,
            membership_after_event: None,
            user_joined_since_event: false,
        };

        assert!(evaluate_event_visibility(&input));
    }

    #[test]
    fn history_visibility_event_visible_when_new_policy_allows() {
        let input = EventVisibilityEvaluationInput {
            event_type: "m.room.history_visibility",
            event_state_key: Some(""),
            requesting_user_id: "@alice:example.org",
            history_visibility_at_event: Some("joined"),
            history_visibility_before_event: Some("joined"),
            history_visibility_after_event: Some("world_readable"),
            membership_at_event: Some("leave"),
            membership_before_event: None,
            membership_after_event: None,
            user_joined_since_event: false,
        };

        assert!(evaluate_event_visibility(&input));
    }

    #[test]
    fn own_membership_event_visible_when_previous_membership_allowed() {
        let input = EventVisibilityEvaluationInput {
            event_type: "m.room.member",
            event_state_key: Some("@alice:example.org"),
            requesting_user_id: "@alice:example.org",
            history_visibility_at_event: Some("joined"),
            history_visibility_before_event: None,
            history_visibility_after_event: None,
            membership_at_event: Some("leave"),
            membership_before_event: Some("join"),
            membership_after_event: Some("leave"),
            user_joined_since_event: false,
        };

        assert!(evaluate_event_visibility(&input));
    }

    #[test]
    fn own_membership_event_visible_when_invited_under_invited_policy() {
        let input = EventVisibilityEvaluationInput {
            event_type: "m.room.member",
            event_state_key: Some("@alice:example.org"),
            requesting_user_id: "@alice:example.org",
            history_visibility_at_event: Some("invited"),
            history_visibility_before_event: None,
            history_visibility_after_event: None,
            membership_at_event: Some("leave"),
            membership_before_event: Some("invite"),
            membership_after_event: Some("leave"),
            user_joined_since_event: false,
        };

        assert!(evaluate_event_visibility(&input));
    }

    #[test]
    fn denies_unrelated_event_when_policy_does_not_allow() {
        let input = EventVisibilityEvaluationInput {
            event_type: "m.room.message",
            event_state_key: None,
            requesting_user_id: "@alice:example.org",
            history_visibility_at_event: Some("joined"),
            history_visibility_before_event: None,
            history_visibility_after_event: None,
            membership_at_event: Some("leave"),
            membership_before_event: None,
            membership_after_event: None,
            user_joined_since_event: false,
        };

        assert!(!evaluate_event_visibility(&input));
    }

    #[test]
    fn shared_policy_allows_after_later_join() {
        let input = EventVisibilityEvaluationInput {
            event_type: "m.room.message",
            event_state_key: None,
            requesting_user_id: "@alice:example.org",
            history_visibility_at_event: Some("shared"),
            history_visibility_before_event: None,
            history_visibility_after_event: None,
            membership_at_event: Some("leave"),
            membership_before_event: None,
            membership_after_event: None,
            user_joined_since_event: true,
        };

        assert!(evaluate_event_visibility(&input));
    }
}
