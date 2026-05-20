use crate::infrastructure::user_identifier::UserIdentifier;
use crate::services::authorization::persistence::access_session_storage_unit::AccessSessionStorageUnit;
use crate::services::authorization::service::AuthorizationService;
use crate::services::rooms::entities::{JoinedMembers, JoinedRoomMember};
use crate::services::rooms::errors::RoomsApplicationError;
use crate::services::rooms::persistence::{JoinedRoomMemberProfile, RoomJoinedMembersRepository};

use super::room_access::require_room_identifier;

pub struct GetJoinedMembersUseCase<'a, Repository>
where
    Repository: RoomJoinedMembersRepository + ?Sized,
{
    room_repository: &'a Repository,
    authorization_service: &'a AuthorizationService,
}

impl<'a, Repository> GetJoinedMembersUseCase<'a, Repository>
where
    Repository: RoomJoinedMembersRepository + ?Sized,
{
    pub fn new(
        room_repository: &'a Repository,
        authorization_service: &'a AuthorizationService,
    ) -> Self {
        Self {
            room_repository,
            authorization_service,
        }
    }

    pub fn execute(
        &self,
        access_session: &AccessSessionStorageUnit,
        room_id: String,
    ) -> Result<JoinedMembers, RoomsApplicationError> {
        require_room_identifier(&room_id)?;

        let mut member_profiles = None;
        let can_view_members = self
            .room_repository
            .fetch_join_context(&room_id, access_session.user_identifier().as_str())
            .map_err(|_| RoomsApplicationError::Internal)?
            .and_then(|value| value.membership_state)
            .as_deref()
            == Some("join");

        if !can_view_members {
            let joined_member_profiles = self
                .room_repository
                .fetch_joined_members_profiles(&room_id)
                .map_err(|_| RoomsApplicationError::Internal)?;

            let mut access_allowed = false;
            for member_profile in &joined_member_profiles {
                let user_identifier = UserIdentifier::try_from(member_profile.user_id.clone())
                    .map_err(|_| RoomsApplicationError::Internal)?;
                let controlled = self
                    .authorization_service
                    .session_controls_user_identifier(access_session, &user_identifier)
                    .map_err(|_| RoomsApplicationError::Internal)?;
                if controlled {
                    access_allowed = true;
                    break;
                }
            }

            if !access_allowed {
                return Err(RoomsApplicationError::Forbidden);
            }
            member_profiles = Some(joined_member_profiles);
        }

        let members = match member_profiles {
            Some(member_profiles) => member_profiles,
            None => self
                .room_repository
                .fetch_joined_members_profiles(&room_id)
                .map_err(|_| RoomsApplicationError::Internal)?,
        };

        Ok(map_joined_members(members))
    }
}

fn map_joined_members(member_profiles: Vec<JoinedRoomMemberProfile>) -> JoinedMembers {
    JoinedMembers {
        joined: member_profiles
            .into_iter()
            .map(|member| {
                (
                    member.user_id,
                    JoinedRoomMember {
                        display_name: member.display_name,
                        avatar_url: member.avatar_url,
                    },
                )
            })
            .collect(),
    }
}
