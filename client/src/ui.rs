use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui;
use tokio::runtime::Runtime;
use tracing::{error, info, warn};

use crate::{
    application::{
        authorization_client::AuthorizationClientService,
        ports::{
            RegisterUserResult, RoomRepository, ServerProfileRepository, SessionRepository,
        },
        rooms_client::RoomsClientService,
        synchronization_client::SynchronizationClientService,
    },
    domain::{
        authorization::{
            AccountKind, AuthenticationDataInfo, LoginType, LoginUserInfo, RegisterUserInfo,
            ServerCredentials, WhoAmIView,
        },
        rooms::{
            CreateRoomInfo, GetRoomEventView, GetRoomMembersQuery, GetRoomMessagesQuery,
            InviteUserInfo, JoinRoomInfo, LeaveRoomInfo, RoomListItem, RoomMessageDirection,
            RoomStateEventView, RoomTimelineEventView, SendReceiptInfo, SetReadMarkersInfo,
        },
        synchronization::SyncQuery,
        synchronization::DefineFilterInfo,
    },
    infrastructure::{
        http::{
            hyper_authorization_api::HyperAuthorizationApi, hyper_rooms_api::HyperRoomsApi,
            hyper_synchronization_api::HyperSynchronizationApi,
        },
        persistence::sqlite_session_repository::SqliteSessionRepository,
    },
};

const PREFERENCE_SELECTED_SERVER_URL: &str = "selected_server_url";
const MESSAGE_PAGE_SIZE: usize = 30;
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(11, 16, 24);
const PANEL: egui::Color32 = egui::Color32::from_rgb(18, 25, 36);
const PANEL_SOFT: egui::Color32 = egui::Color32::from_rgb(25, 34, 48);
const PANEL_HOVER: egui::Color32 = egui::Color32::from_rgb(35, 47, 66);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(89, 214, 190);
const ACCENT_STRONG: egui::Color32 = egui::Color32::from_rgb(247, 198, 103);
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(151, 164, 184);
const DANGER: egui::Color32 = egui::Color32::from_rgb(255, 103, 124);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Welcome,
    ChooseFlow,
    RegisterForm,
    Workspace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum UsernameState {
    Unknown,
    Available,
    Unavailable(String),
}

#[derive(Clone, Debug, Default)]
enum LoadState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Error(String),
}

#[derive(Clone, Debug)]
struct JoinedMemberView {
    user_id: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Clone, Debug)]
struct RoomPolicyProfile {
    visibility: String,
    preset: String,
    is_direct: bool,
    owner_user_id: Option<String>,
    can_invite: bool,
    can_rename_name: bool,
}

pub struct ClientDesktopApplication {
    runtime: Runtime,
    session_repository: Arc<dyn SessionRepository>,
    server_profile_repository: Arc<dyn ServerProfileRepository>,
    room_repository: Arc<dyn RoomRepository>,
    screen: Screen,
    servers: Vec<String>,
    selected_server_index: usize,
    accounts_for_server: Vec<ServerCredentials>,
    selected_account_index: usize,
    show_add_server_form: bool,
    new_server_url_input: String,
    server_url_input: String,
    username_input: String,
    password_input: String,
    available_auth_flows: Vec<String>,
    selected_auth_flow_index: usize,
    username_state: UsernameState,
    status_message: String,
    whoami: Option<WhoAmIView>,
    rooms: Vec<RoomListItem>,
    selected_room_index: Option<usize>,
    show_create_room_form: bool,
    join_room_id_input: String,
    join_reason_input: String,
    new_room_name: String,
    new_room_topic: String,
    new_room_visibility_index: usize,
    new_room_preset_index: usize,
    new_room_is_direct: bool,
    room_state_events: Vec<RoomStateEventView>,
    room_messages: Vec<RoomTimelineEventView>,
    room_members: Vec<JoinedMemberView>,
    room_members_status: LoadState,
    room_members_timeline_events: Vec<RoomStateEventView>,
    room_members_timeline_status: LoadState,
    room_state_status: LoadState,
    room_messages_status: LoadState,
    next_messages_from_token: Option<String>,
    room_policy_profiles: BTreeMap<String, RoomPolicyProfile>,
    sync_enabled: bool,
    sync_interval_seconds: f32,
    sync_timeout_milliseconds: u64,
    next_sync_batch_token: Option<String>,
    last_sync_poll_started_at: Option<Instant>,
    sync_filter_definition_input: String,
    sync_filter_id_input: String,
    sync_filter_lookup_output: Option<serde_json::Value>,
    composer_input: String,
    invite_user_id_input: String,
    invite_reason_input: String,
    room_membership_filter_input: String,
    room_not_membership_filter_input: String,
    room_members_at_token_input: String,
    receipt_event_id_input: String,
    receipt_thread_id_input: String,
    receipt_type_index: usize,
    event_lookup_input: String,
    looked_up_event: Option<GetRoomEventView>,
    state_event_type_input: String,
    state_key_input: String,
    state_content_input: String,
    looked_up_state_value: Option<serde_json::Value>,
    next_transaction_counter: u64,
    last_logged_status_message: String,
}

impl ClientDesktopApplication {
    pub fn new(repository: Arc<SqliteSessionRepository>) -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|error| error.to_string())?;
        let session_repository: Arc<dyn SessionRepository> = repository.clone();
        let server_profile_repository: Arc<dyn ServerProfileRepository> = repository.clone();
        let room_repository: Arc<dyn RoomRepository> = repository.clone();

        let mut application = Self {
            runtime,
            session_repository,
            server_profile_repository,
            room_repository,
            screen: Screen::Welcome,
            servers: Vec::new(),
            selected_server_index: 0,
            accounts_for_server: Vec::new(),
            selected_account_index: 0,
            show_add_server_form: false,
            new_server_url_input: String::new(),
            server_url_input: "http://127.0.0.1:8080".to_owned(),
            username_input: String::new(),
            password_input: String::new(),
            available_auth_flows: Vec::new(),
            selected_auth_flow_index: 0,
            username_state: UsernameState::Unknown,
            status_message: "Select server and account, then login".to_owned(),
            whoami: None,
            rooms: Vec::new(),
            selected_room_index: None,
            show_create_room_form: false,
            join_room_id_input: String::new(),
            join_reason_input: String::new(),
            new_room_name: String::new(),
            new_room_topic: String::new(),
            new_room_visibility_index: 0,
            new_room_preset_index: 0,
            new_room_is_direct: false,
            room_state_events: Vec::new(),
            room_messages: Vec::new(),
            room_members: Vec::new(),
            room_members_status: LoadState::Idle,
            room_members_timeline_events: Vec::new(),
            room_members_timeline_status: LoadState::Idle,
            room_state_status: LoadState::Idle,
            room_messages_status: LoadState::Idle,
            next_messages_from_token: None,
            room_policy_profiles: BTreeMap::new(),
            sync_enabled: true,
            sync_interval_seconds: 2.0,
            sync_timeout_milliseconds: 1500,
            next_sync_batch_token: None,
            last_sync_poll_started_at: None,
            sync_filter_definition_input: format!(
                "{}",
                serde_json::json!({
                    "room": {
                        "timeline": {
                            "limit": MESSAGE_PAGE_SIZE
                        }
                    }
                })
            ),
            sync_filter_id_input: String::new(),
            sync_filter_lookup_output: None,
            composer_input: String::new(),
            invite_user_id_input: String::new(),
            invite_reason_input: String::new(),
            room_membership_filter_input: String::new(),
            room_not_membership_filter_input: String::new(),
            room_members_at_token_input: String::new(),
            receipt_event_id_input: String::new(),
            receipt_thread_id_input: String::new(),
            receipt_type_index: 0,
            event_lookup_input: String::new(),
            looked_up_event: None,
            state_event_type_input: "m.room.name".to_owned(),
            state_key_input: String::new(),
            state_content_input: "{\n  \"name\": \"New Room Name\"\n}".to_owned(),
            looked_up_state_value: None,
            next_transaction_counter: 0,
            last_logged_status_message: String::new(),
        };

        application.reload_servers();
        application.log_status_message_if_changed();
        Ok(application)
    }

    fn log_status_message_if_changed(&mut self) {
        if self.status_message == self.last_logged_status_message {
            return;
        }

        append_status_message_to_log_file(&self.status_message);

        let status_lowercase = self.status_message.to_ascii_lowercase();
        if status_lowercase.contains("failed")
            || status_lowercase.contains("error")
            || status_lowercase.contains("invalid")
        {
            error!(status_message = %self.status_message, "ui status updated");
        } else if status_lowercase.contains("required")
            || status_lowercase.contains("select")
            || status_lowercase.contains("no ")
        {
            warn!(status_message = %self.status_message, "ui status updated");
        } else {
            info!(status_message = %self.status_message, "ui status updated");
        }

        self.last_logged_status_message = self.status_message.clone();
    }

    fn normalized_server_url(&self) -> String {
        normalize_server_url_input(&self.server_url_input).unwrap_or_default()
    }

    fn selected_server(&self) -> Option<&str> {
        self.servers
            .get(self.selected_server_index)
            .map(std::string::String::as_str)
    }

    fn selected_account(&self) -> Option<&ServerCredentials> {
        self.accounts_for_server.get(self.selected_account_index)
    }

    fn selected_room(&self) -> Option<&RoomListItem> {
        self.selected_room_index.and_then(|index| self.rooms.get(index))
    }

    fn build_authorization_service(&self) -> AuthorizationClientService {
        let server_url = self
            .selected_server()
            .map(str::to_owned)
            .unwrap_or_else(|| self.normalized_server_url());
        let api = Arc::new(HyperAuthorizationApi::new(server_url.clone()));
        AuthorizationClientService::new(server_url, api, Arc::clone(&self.session_repository))
    }

    fn build_rooms_service(&self) -> RoomsClientService {
        let server_url = self
            .selected_server()
            .map(str::to_owned)
            .unwrap_or_else(|| self.normalized_server_url());
        let rooms_api = Arc::new(HyperRoomsApi::new(server_url.clone()));
        RoomsClientService::new(
            server_url,
            rooms_api,
            Arc::clone(&self.session_repository),
            Arc::clone(&self.room_repository),
        )
    }

    fn build_synchronization_service(&self) -> SynchronizationClientService {
        let server_url = self
            .selected_server()
            .map(str::to_owned)
            .unwrap_or_else(|| self.normalized_server_url());
        let synchronization_api = Arc::new(HyperSynchronizationApi::new(server_url.clone()));
        SynchronizationClientService::new(
            server_url,
            synchronization_api,
            Arc::clone(&self.session_repository),
        )
    }

    fn preference_selected_username_key(server_url: &str) -> String {
        format!("selected_account_username:{server_url}")
    }

    fn persist_selected_server(&mut self) {
        if let Some(server_url) = self.selected_server().map(str::to_owned) {
            let _ = self.runtime.block_on(
                self.server_profile_repository
                    .set_preference(PREFERENCE_SELECTED_SERVER_URL, &server_url),
            );
        }
    }

    fn persist_selected_account(&mut self) {
        let Some(server_url) = self.selected_server().map(str::to_owned) else {
            return;
        };
        let Some(username) = self.selected_account().map(|account| account.username.clone()) else {
            return;
        };

        let key = Self::preference_selected_username_key(&server_url);
        let _ = self
            .runtime
            .block_on(self.server_profile_repository.set_preference(&key, &username));
    }

    fn reload_servers(&mut self) {
        let selected_server_preference = self.runtime.block_on(
            self.server_profile_repository
                .get_preference(PREFERENCE_SELECTED_SERVER_URL),
        );

        match self
            .runtime
            .block_on(self.server_profile_repository.list_server_credentials())
        {
            Ok(credentials) => {
                let mut servers: Vec<String> = credentials.into_iter().map(|x| x.server_url).collect();
                servers.sort();
                servers.dedup();
                self.servers = servers;

                if let Ok(Some(preferred_server)) = selected_server_preference {
                    if let Some(index) = self
                        .servers
                        .iter()
                        .position(|server| server == &preferred_server)
                    {
                        self.selected_server_index = index;
                    } else if self.selected_server_index >= self.servers.len() {
                        self.selected_server_index = 0;
                    }
                } else if self.selected_server_index >= self.servers.len() {
                    self.selected_server_index = 0;
                }

                if let Some(server) = self.selected_server().map(str::to_owned) {
                    self.server_url_input = server;
                }
                self.reload_accounts_for_selected_server();
            }
            Err(error) => {
                self.status_message = format!("Failed to load servers: {error}");
            }
        }
    }

    fn reload_accounts_for_selected_server(&mut self) {
        let Some(server) = self.selected_server().map(str::to_owned) else {
            self.accounts_for_server.clear();
            self.selected_account_index = 0;
            return;
        };

        let key = Self::preference_selected_username_key(&server);
        let selected_account_preference = self
            .runtime
            .block_on(self.server_profile_repository.get_preference(&key));

        match self
            .runtime
            .block_on(self.server_profile_repository.list_server_accounts(&server))
        {
            Ok(accounts) => {
                self.accounts_for_server = accounts;

                if let Ok(Some(preferred_username)) = selected_account_preference {
                    if let Some(index) = self
                        .accounts_for_server
                        .iter()
                        .position(|account| account.username == preferred_username)
                    {
                        self.selected_account_index = index;
                    } else if self.selected_account_index >= self.accounts_for_server.len() {
                        self.selected_account_index = 0;
                    }
                } else if self.selected_account_index >= self.accounts_for_server.len() {
                    self.selected_account_index = 0;
                }

                if let Some(account) = self.accounts_for_server.get(self.selected_account_index) {
                    self.username_input = account.username.clone();
                    self.password_input = account.password.clone();
                }
            }
            Err(error) => {
                self.status_message = format!("Failed to load accounts: {error}");
            }
        }
    }

    fn add_server(&mut self) {
        let server_url = match normalize_server_url_input(&self.new_server_url_input) {
            Ok(url) => url,
            Err(message) => {
                self.status_message = message;
                return;
            }
        };
        if server_url.is_empty() {
            self.status_message = "Server URL required".to_owned();
            return;
        }

        if !self.servers.iter().any(|known_server| known_server == &server_url) {
            self.servers.push(server_url.clone());
            self.servers.sort();
        }
        if let Some(index) = self.servers.iter().position(|known_server| known_server == &server_url) {
            self.selected_server_index = index;
            self.server_url_input = server_url;
            self.persist_selected_server();
            self.reload_accounts_for_selected_server();
        }

        self.new_server_url_input.clear();
        self.show_add_server_form = false;
        self.status_message = "Server added".to_owned();
    }

    fn save_selected_account_credentials(&mut self) {
        let Some(server_url) = self.selected_server().map(str::to_owned) else {
            self.status_message = "Select server first".to_owned();
            return;
        };

        if self.username_input.trim().is_empty() {
            self.status_message = "Username required".to_owned();
            return;
        }

        let credentials = ServerCredentials {
            server_url: server_url.clone(),
            username: self.username_input.trim().to_owned(),
            password: self.password_input.clone(),
        };
        match self.runtime.block_on(
            self.server_profile_repository
                .upsert_server_credentials(&credentials),
        ) {
            Ok(_) => {
                self.status_message = "Account saved".to_owned();
                self.reload_accounts_for_selected_server();
                if let Some(index) = self
                    .accounts_for_server
                    .iter()
                    .position(|account| account.username == credentials.username)
                {
                    self.selected_account_index = index;
                }
                self.persist_selected_account();
            }
            Err(error) => self.status_message = format!("Failed to save account: {error}"),
        }
    }

    fn run_login_selected_account(&mut self) {
        let Some(account) = self.accounts_for_server.get(self.selected_account_index).cloned() else {
            self.status_message = "Select account first".to_owned();
            return;
        };

        self.username_input = account.username;
        self.password_input = account.password;

        let service = self.build_authorization_service();
        let request = LoginUserInfo {
            login_type: LoginType::Password,
            identifier: None,
            user: Some(self.username_input.trim().to_owned()),
            password: self.password_input.clone(),
            device_identifier: None,
            refresh_token: true,
            token: None,
            medium: None,
            address: None,
            initial_device_display_name: Some("Gridstack Desktop".to_owned()),
        };

        match self.runtime.block_on(service.login_user(&request)) {
            Ok(_) => {
                self.persist_selected_server();
                self.persist_selected_account();
                self.load_whoami();
            }
            Err(error) => self.status_message = format!("Login failed: {error}"),
        }
    }

    fn load_whoami(&mut self) {
        let service = self.build_authorization_service();
        match self.runtime.block_on(service.who_am_i_from_saved_session()) {
            Ok(view) => {
                self.whoami = Some(view);
                self.screen = Screen::Workspace;
                self.status_message = "Login successful".to_owned();
                self.load_rooms();
            }
            Err(error) => {
                self.status_message = format!("Failed to load whoami: {error}");
            }
        }
    }

    fn load_rooms(&mut self) {
        let rooms_service = self.build_rooms_service();
        match self.runtime.block_on(rooms_service.get_joined_rooms()) {
            Ok(joined_rooms) => {
                for room_id in joined_rooms.joined_rooms {
                    let _ = self
                        .runtime
                        .block_on(rooms_service.upsert_cached_room(&room_id, None, None));
                }
            }
            Err(error) => {
                self.status_message = format!(
                    "Failed to refresh joined rooms from server. Showing cached rooms: {error}"
                );
            }
        }

        match self.runtime.block_on(rooms_service.list_cached_rooms()) {
            Ok(items) => {
                let previously_selected_room_id =
                    self.selected_room().map(|room| room.room_id.clone());
                self.rooms = items;
                let current_room_ids = self
                    .rooms
                    .iter()
                    .map(|room| room.room_id.clone())
                    .collect::<Vec<_>>();
                self.room_policy_profiles
                    .retain(|room_id, _| current_room_ids.iter().any(|current_id| current_id == room_id));

                self.selected_room_index = previously_selected_room_id
                    .and_then(|room_id| self.rooms.iter().position(|room| room.room_id == room_id))
                    .or_else(|| (!self.rooms.is_empty()).then_some(0));

                if self.selected_room_index.is_none() {
                    self.room_state_events.clear();
                    self.room_messages.clear();
                    self.next_messages_from_token = None;
                }
            }
            Err(error) => {
                self.status_message = format!("Failed to load cached rooms: {error}");
            }
        }
    }

    fn start_registration(&mut self) {
        let service = self.build_authorization_service();
        match self.runtime.block_on(service.get_login_flows()) {
            Ok(view) => {
                self.available_auth_flows = view.flows.into_iter().map(|flow| flow.type_field).collect();
                if self.available_auth_flows.is_empty() {
                    self.available_auth_flows.push("m.login.password".to_owned());
                }
                self.selected_auth_flow_index = 0;
                self.username_state = UsernameState::Unknown;
                self.screen = Screen::ChooseFlow;
            }
            Err(error) => self.status_message = format!("Failed to load auth flows: {error}"),
        }
    }

    fn check_username(&mut self) {
        let service = self.build_authorization_service();
        match self
            .runtime
            .block_on(service.check_username_available(self.username_input.trim()))
        {
            Ok(view) => {
                self.username_state = if view.available {
                    UsernameState::Available
                } else {
                    UsernameState::Unavailable("Username is not available".to_owned())
                };
            }
            Err(error) => self.username_state = UsernameState::Unavailable(error.to_string()),
        }
    }

    fn submit_registration(&mut self) {
        if self.username_state != UsernameState::Available {
            self.status_message = "Check username availability before register".to_owned();
            return;
        }

        let service = self.build_authorization_service();
        let first_request = self.build_register_request(None);
        match self
            .runtime
            .block_on(service.register_user(AccountKind::User, &first_request))
        {
            Ok(RegisterUserResult::Registered(_)) => {
                self.save_selected_account_credentials();
                self.persist_selected_server();
                self.persist_selected_account();
                self.load_whoami();
            }
            Ok(RegisterUserResult::AuthenticationRequired(uiaa)) => {
                let flow = self
                    .available_auth_flows
                    .get(self.selected_auth_flow_index)
                    .cloned()
                    .unwrap_or_else(|| "m.login.password".to_owned());
                let second_request = self.build_register_request(Some(AuthenticationDataInfo {
                    session: Some(uiaa.session),
                    authentication_type: Some(flow),
                    data: BTreeMap::new(),
                }));

                match self
                    .runtime
                    .block_on(service.register_user(AccountKind::User, &second_request))
                {
                    Ok(RegisterUserResult::Registered(_)) => {
                        self.save_selected_account_credentials();
                        self.persist_selected_server();
                        self.persist_selected_account();
                        self.load_whoami();
                    }
                    Ok(RegisterUserResult::AuthenticationRequired(_)) => {
                        self.status_message = "Additional auth steps required".to_owned();
                    }
                    Err(error) => self.status_message = format!("Registration failed: {error}"),
                }
            }
            Err(error) => self.status_message = format!("Registration failed: {error}"),
        }
    }

    fn build_register_request(
        &self,
        authentication: Option<AuthenticationDataInfo>,
    ) -> RegisterUserInfo {
        RegisterUserInfo {
            authentication,
            device_identifier: None,
            inhibit_login: false,
            initial_device_display_name: Some("Gridstack Desktop".to_owned()),
            password: Some(self.password_input.clone()),
            refresh_token: true,
            username: Some(self.username_input.trim().to_owned()),
        }
    }

    fn run_logout(&mut self) {
        let service = self.build_authorization_service();
        match self.runtime.block_on(service.logout_from_saved_session()) {
            Ok(_) => {
                self.screen = Screen::Welcome;
                self.status_message = "Logged out".to_owned();
                self.rooms.clear();
                self.selected_room_index = None;
                self.room_state_events.clear();
                self.room_messages.clear();
                self.room_state_status = LoadState::Idle;
                self.room_messages_status = LoadState::Idle;
                self.next_messages_from_token = None;
                self.whoami = None;
                self.composer_input.clear();
            }
            Err(error) => self.status_message = format!("Logout failed: {error}"),
        }
    }

    fn create_room(&mut self) {
        let rooms_service = self.build_rooms_service();
        let visibility = Self::visibility_options()
            .get(self.new_room_visibility_index)
            .copied()
            .unwrap_or("private")
            .to_owned();
        let preset = Self::available_presets_for_visibility(visibility.as_str())
            .get(self.new_room_preset_index)
            .copied()
            .unwrap_or("private_chat")
            .to_owned();
        let request = CreateRoomInfo {
            name: if self.new_room_name.trim().is_empty() {
                None
            } else {
                Some(self.new_room_name.trim().to_owned())
            },
            topic: if self.new_room_topic.trim().is_empty() {
                None
            } else {
                Some(self.new_room_topic.trim().to_owned())
            },
            visibility: Some(visibility),
            preset: Some(preset),
            room_alias_name: None,
            room_version: None,
            is_direct: Some(self.new_room_is_direct),
            creation_content: None,
            invite: None,
            invite_3pid: None,
            power_level_content_override: None,
        };

        match self.runtime.block_on(rooms_service.create_room(&request)) {
            Ok(view) => {
                self.status_message = format!("Room created: {}", view.room_id);
                self.new_room_name.clear();
                self.new_room_topic.clear();
                self.show_create_room_form = false;
                self.load_rooms();
                self.select_room_by_id(&view.room_id);
            }
            Err(error) => {
                self.status_message = format!("Create room failed: {error}");
            }
        }
    }

    fn join_room_by_id(&mut self) {
        let room_id = self.join_room_id_input.trim().to_owned();
        if room_id.is_empty() {
            self.status_message = "Room ID is required for join".to_owned();
            return;
        }

        let request = JoinRoomInfo {
            reason: (!self.join_reason_input.trim().is_empty())
                .then(|| self.join_reason_input.trim().to_owned()),
        };

        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.join_room_by_id(&room_id, &request))
        {
            Ok(view) => {
                self.status_message = format!("Joined room: {}", view.room_id);
                self.join_room_id_input.clear();
                self.join_reason_input.clear();
                self.load_rooms();
                self.select_room_by_id(&view.room_id);
            }
            Err(error) => {
                self.status_message = format!("Join room failed: {error}");
            }
        }
    }

    fn leave_selected_room(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room first".to_owned();
            return;
        };

        let request = LeaveRoomInfo {
            reason: (!self.join_reason_input.trim().is_empty())
                .then(|| self.join_reason_input.trim().to_owned()),
        };
        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.leave_room_by_id(&room_id, &request))
        {
            Ok(_) => {
                self.status_message = format!("Left room: {room_id}");
                self.load_rooms();
                self.room_state_events.clear();
                self.room_messages.clear();
                self.room_members_timeline_events.clear();
                self.room_state_status = LoadState::Idle;
                self.room_messages_status = LoadState::Idle;
                self.room_members_timeline_status = LoadState::Idle;
                self.next_messages_from_token = None;
            }
            Err(error) => self.status_message = format!("Leave room failed: {error}"),
        }
    }

    fn select_room_by_id(&mut self, room_id: &str) {
        if self.set_selected_room_index_by_id(room_id) {
            self.looked_up_event = None;
            self.looked_up_state_value = None;
            self.room_members.clear();
            self.room_members_status = LoadState::Idle;
            self.room_members_timeline_events.clear();
            self.room_members_timeline_status = LoadState::Idle;
            self.load_selected_room_details();
        }
    }

    fn set_selected_room_index_by_id(&mut self, room_id: &str) -> bool {
        if let Some(index) = self.rooms.iter().position(|room| room.room_id == room_id) {
            self.selected_room_index = Some(index);
            return true;
        }
        false
    }

    fn load_selected_room_details(&mut self) {
        self.load_selected_room_state();
        self.load_selected_room_messages(None, true);
    }

    fn load_selected_room_state(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.room_state_status = LoadState::Idle;
            return;
        };

        self.room_state_status = LoadState::Loading;
        let rooms_service = self.build_rooms_service();
        match self.runtime.block_on(rooms_service.get_room_state(&room_id)) {
            Ok(events) => {
                self.room_state_events = events;
                self.room_state_status = LoadState::Loaded;
                let room_state_events_snapshot = self.room_state_events.clone();
                self.update_room_policy_profile_from_state(&room_id, &room_state_events_snapshot);

                let (name, topic) = extract_room_name_and_topic(&self.room_state_events);
                if name.is_some() || topic.is_some() {
                    let _ = self.runtime.block_on(rooms_service.upsert_cached_room(
                        &room_id,
                        name,
                        topic,
                    ));
                    self.load_rooms();
                    let _ = self.set_selected_room_index_by_id(&room_id);
                }
            }
            Err(error) => {
                self.room_state_events.clear();
                self.room_state_status = LoadState::Error(error.to_string());
            }
        }
    }

    fn update_room_policy_profile_from_state(
        &mut self,
        room_id: &str,
        room_state_events: &[RoomStateEventView],
    ) {
        let whoami_user_id = self.whoami.as_ref().map(|value| value.user_id.as_str());
        let profile = derive_room_policy_profile(room_state_events, whoami_user_id);
        self.room_policy_profiles.insert(room_id.to_owned(), profile);
    }

    fn selected_room_policy_profile(&self) -> Option<&RoomPolicyProfile> {
        self.selected_room()
            .and_then(|room| self.room_policy_profiles.get(&room.room_id))
    }

    fn load_selected_room_messages(&mut self, from_token: Option<String>, replace: bool) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.room_messages_status = LoadState::Idle;
            return;
        };

        self.room_messages_status = LoadState::Loading;
        let query = GetRoomMessagesQuery {
            from_token,
            to_token: None,
            direction: RoomMessageDirection::Backward,
            limit: MESSAGE_PAGE_SIZE,
            filter: None,
        };
        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.get_room_messages(&room_id, &query))
        {
            Ok(page) => {
                self.next_messages_from_token = page.end;
                if replace {
                    self.room_messages = page.chunk;
                } else {
                    self.room_messages.extend(page.chunk);
                }
                self.sort_room_messages_for_chat();
                self.room_messages_status = LoadState::Loaded;
            }
            Err(error) => {
                self.room_messages_status = LoadState::Error(error.to_string());
            }
        }
    }

    fn sort_room_messages_for_chat(&mut self) {
        self.room_messages.sort_by(|left, right| {
            left.origin_server_ts
                .cmp(&right.origin_server_ts)
                .then_with(|| left.event_id.cmp(&right.event_id))
        });
        self.room_messages
            .dedup_by(|left, right| left.event_id == right.event_id);
    }

    fn load_older_messages(&mut self) {
        let Some(token) = self.next_messages_from_token.clone() else {
            self.status_message = "No older messages token available for this room".to_owned();
            return;
        };
        self.load_selected_room_messages(Some(token), false);
    }

    fn send_message(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room before sending a message".to_owned();
            return;
        };

        let message_body = self.composer_input.trim().to_owned();
        if message_body.is_empty() {
            self.status_message = "Message text is required".to_owned();
            return;
        }

        let transaction_id = self.build_transaction_id();
        let content = serde_json::json!({
            "msgtype": "m.text",
            "body": message_body,
        });
        let rooms_service = self.build_rooms_service();

        match self.runtime.block_on(rooms_service.send_room_message_event(
            &room_id,
            "m.room.message",
            &transaction_id,
            &content,
        )) {
            Ok(response) => {
                self.status_message = format!("Message sent ({})", response.event_id);
                self.composer_input.clear();
                self.load_selected_room_messages(None, true);
            }
            Err(error) => {
                self.status_message = format!("Send message failed: {error}");
            }
        }
    }

    fn invite_user_to_selected_room(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room before inviting users".to_owned();
            return;
        };
        if let Some(policy_profile) = self.selected_room_policy_profile()
            && !policy_profile.can_invite
        {
            self.status_message = format!(
                "Invite blocked by room policy: only `{}` can invite in this private room",
                policy_profile
                    .owner_user_id
                    .as_deref()
                    .unwrap_or("room owner")
            );
            return;
        }
        let invited_user_id = self.invite_user_id_input.trim().to_owned();
        if invited_user_id.is_empty() {
            self.status_message = "Invite user id is required".to_owned();
            return;
        }

        let request = InviteUserInfo {
            user_id: Some(invited_user_id),
            reason: (!self.invite_reason_input.trim().is_empty())
                .then(|| self.invite_reason_input.trim().to_owned()),
            id_server: None,
            id_access_token: None,
            medium: None,
            address: None,
        };
        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.invite_user_to_room(&room_id, &request))
        {
            Ok(_) => {
                self.status_message = "Invite request accepted".to_owned();
                self.invite_user_id_input.clear();
                self.invite_reason_input.clear();
            }
            Err(error) => {
                self.status_message = format!("Invite failed: {error}");
            }
        }
    }

    fn load_selected_room_members(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.room_members_status = LoadState::Idle;
            return;
        };
        self.room_members_status = LoadState::Loading;
        let rooms_service = self.build_rooms_service();

        match self.runtime.block_on(rooms_service.get_joined_members(&room_id)) {
            Ok(view) => {
                self.room_members = view
                    .joined
                    .into_iter()
                    .map(|(user_id, member)| JoinedMemberView {
                        user_id,
                        display_name: member.display_name,
                        avatar_url: member.avatar_url,
                    })
                    .collect();
                self.room_members.sort_by(|a, b| a.user_id.cmp(&b.user_id));
                self.room_members_status = LoadState::Loaded;
            }
            Err(error) => {
                self.room_members.clear();
                self.room_members_status = LoadState::Error(error.to_string());
            }
        }
    }

    fn load_selected_room_members_with_filter(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.room_members_timeline_status = LoadState::Idle;
            return;
        };
        self.room_members_timeline_status = LoadState::Loading;
        let query = GetRoomMembersQuery {
            at_token: (!self.room_members_at_token_input.trim().is_empty())
                .then(|| self.room_members_at_token_input.trim().to_owned()),
            membership: (!self.room_membership_filter_input.trim().is_empty())
                .then(|| self.room_membership_filter_input.trim().to_owned()),
            not_membership: (!self.room_not_membership_filter_input.trim().is_empty())
                .then(|| self.room_not_membership_filter_input.trim().to_owned()),
        };
        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.get_room_members(&room_id, &query))
        {
            Ok(view) => {
                self.room_members_timeline_events = view.chunk;
                self.room_members_timeline_events.sort_by(|left, right| {
                    left.origin_server_ts
                        .cmp(&right.origin_server_ts)
                        .then_with(|| left.event_id.cmp(&right.event_id))
                });
                self.room_members_timeline_status = LoadState::Loaded;
            }
            Err(error) => {
                self.room_members_timeline_events.clear();
                self.room_members_timeline_status = LoadState::Error(error.to_string());
            }
        }
    }

    fn lookup_room_event(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room before looking up events".to_owned();
            return;
        };
        let event_id = self.event_lookup_input.trim().to_owned();
        if event_id.is_empty() {
            self.status_message = "Event id is required".to_owned();
            return;
        }

        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.get_room_event(&room_id, &event_id))
        {
            Ok(view) => {
                self.looked_up_event = Some(view);
                self.status_message = "Event fetched".to_owned();
            }
            Err(error) => {
                self.looked_up_event = None;
                self.status_message = format!("Event lookup failed: {error}");
            }
        }
    }

    fn lookup_state_with_key(&mut self, request_full_event: bool) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room before reading state".to_owned();
            return;
        };
        let event_type = self.state_event_type_input.trim().to_owned();
        if event_type.is_empty() {
            self.status_message = "State event type is required".to_owned();
            return;
        }
        let state_key = self.state_key_input.trim();
        let state_key_option = if state_key.is_empty() {
            None
        } else {
            Some(state_key)
        };

        let rooms_service = self.build_rooms_service();
        match self.runtime.block_on(rooms_service.get_room_state_with_key(
            &room_id,
            &event_type,
            state_key_option,
            request_full_event,
        )) {
            Ok(value) => {
                self.looked_up_state_value = Some(value);
                self.status_message = "Room state fetched".to_owned();
            }
            Err(error) => {
                self.looked_up_state_value = None;
                self.status_message = format!("Room state lookup failed: {error}");
            }
        }
    }

    fn set_state_with_key(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room before setting state".to_owned();
            return;
        };
        let event_type = self.state_event_type_input.trim().to_owned();
        let state_key = self.state_key_input.trim().to_owned();
        if event_type == "m.room.name"
            && let Some(policy_profile) = self.selected_room_policy_profile()
            && !policy_profile.can_rename_name
        {
            self.status_message = format!(
                "Rename blocked by room policy for preset `{}`",
                policy_profile.preset
            );
            return;
        }
        if event_type.is_empty() || state_key.is_empty() {
            self.status_message = "State event type and non-empty state key are required".to_owned();
            return;
        }

        let content: serde_json::Value = match serde_json::from_str(&self.state_content_input) {
            Ok(content) => content,
            Err(error) => {
                self.status_message = format!("Invalid JSON for state content: {error}");
                return;
            }
        };
        let rooms_service = self.build_rooms_service();
        match self.runtime.block_on(rooms_service.set_room_state_with_key(
            &room_id,
            &event_type,
            &state_key,
            &content,
        )) {
            Ok(response) => {
                self.status_message = format!("State updated ({})", response.event_id);
                self.load_selected_room_state();
            }
            Err(error) => {
                self.status_message = format!("Set state failed: {error}");
            }
        }
    }

    fn mark_selected_room_as_read(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room before marking as read".to_owned();
            return;
        };
        let Some(latest_event_id) = self
            .room_messages
            .iter()
            .max_by_key(|event| event.origin_server_ts)
            .map(|event| event.event_id.clone())
        else {
            self.status_message = "No message event available to mark read".to_owned();
            return;
        };

        let request = SetReadMarkersInfo {
            fully_read_event_id: Some(latest_event_id.clone()),
            read_event_id: Some(latest_event_id),
            private_read_event_id: None,
        };
        let rooms_service = self.build_rooms_service();
        match self
            .runtime
            .block_on(rooms_service.set_room_read_markers(&room_id, &request))
        {
            Ok(_) => {
                self.status_message = "Read markers updated".to_owned();
            }
            Err(error) => {
                self.status_message = format!("Failed to set read markers: {error}");
            }
        }
    }

    fn send_receipt_for_selected_room(&mut self) {
        let Some(room_id) = self.selected_room().map(|room| room.room_id.clone()) else {
            self.status_message = "Select a room before sending receipt".to_owned();
            return;
        };
        let resolved_event_id = if !self.receipt_event_id_input.trim().is_empty() {
            self.receipt_event_id_input.trim().to_owned()
        } else if let Some(event_id) = self
            .room_messages
            .iter()
            .max_by_key(|event| event.origin_server_ts)
            .map(|event| event.event_id.clone())
        {
            event_id
        } else {
            self.status_message = "Receipt event id is required (or load messages first)".to_owned();
            return;
        };
        let receipt_type = Self::receipt_type_options()
            .get(self.receipt_type_index)
            .copied()
            .unwrap_or("m.read");
        let request = SendReceiptInfo {
            thread_id: (!self.receipt_thread_id_input.trim().is_empty())
                .then(|| self.receipt_thread_id_input.trim().to_owned()),
        };
        let rooms_service = self.build_rooms_service();
        match self.runtime.block_on(rooms_service.send_room_receipt(
            &room_id,
            receipt_type,
            &resolved_event_id,
            &request,
        )) {
            Ok(_) => {
                self.status_message = format!(
                    "Receipt sent: type={receipt_type}, event={resolved_event_id}"
                );
            }
            Err(error) => {
                self.status_message = format!("Send receipt failed: {error}");
            }
        }
    }

    fn run_periodic_sync_if_due(&mut self) {
        if self.screen != Screen::Workspace || !self.sync_enabled {
            return;
        }
        let sync_every = Duration::from_secs_f32(self.sync_interval_seconds.max(0.5));
        if let Some(last_poll_started_at) = self.last_sync_poll_started_at
            && last_poll_started_at.elapsed() < sync_every
        {
            return;
        }
        self.last_sync_poll_started_at = Some(Instant::now());
        self.run_single_sync_poll();
    }

    fn run_single_sync_poll(&mut self) {
        let synchronization_service = self.build_synchronization_service();
        let sync_query = SyncQuery {
            filter: Some(
                serde_json::json!({
                    "room": {
                        "timeline": {
                            "limit": MESSAGE_PAGE_SIZE
                        }
                    }
                })
                .to_string(),
            ),
            since: self.next_sync_batch_token.clone(),
            full_state: false,
            set_presence: Some("online".to_owned()),
            timeout_milliseconds: Some(self.sync_timeout_milliseconds),
            use_state_after: false,
        };
        match self.runtime.block_on(synchronization_service.sync(&sync_query)) {
            Ok(sync_response) => {
                self.next_sync_batch_token = Some(sync_response.next_batch.clone());
                self.apply_sync_response(sync_response);
            }
            Err(error) => {
                self.status_message = format!("Sync failed: {error}");
            }
        }
    }

    fn define_sync_filter(&mut self) {
        let filter_payload: serde_json::Value =
            match serde_json::from_str(&self.sync_filter_definition_input) {
                Ok(payload) => payload,
                Err(error) => {
                    self.status_message = format!("Invalid sync filter JSON: {error}");
                    return;
                }
            };
        let synchronization_service = self.build_synchronization_service();
        match self
            .runtime
            .block_on(synchronization_service.define_filter(&DefineFilterInfo {
                payload: filter_payload,
            })) {
            Ok(view) => {
                self.sync_filter_id_input = view.filter_id.clone();
                self.status_message = format!("Sync filter created: {}", view.filter_id);
            }
            Err(error) => {
                self.status_message = format!("Define sync filter failed: {error}");
            }
        }
    }

    fn fetch_sync_filter(&mut self) {
        let filter_id = self.sync_filter_id_input.trim().to_owned();
        if filter_id.is_empty() {
            self.status_message = "Filter id is required".to_owned();
            return;
        }
        let synchronization_service = self.build_synchronization_service();
        match self
            .runtime
            .block_on(synchronization_service.get_filter(&filter_id))
        {
            Ok(value) => {
                self.sync_filter_lookup_output = Some(value);
                self.status_message = "Sync filter fetched".to_owned();
            }
            Err(error) => {
                self.sync_filter_lookup_output = None;
                self.status_message = format!("Get sync filter failed: {error}");
            }
        }
    }

    fn apply_sync_response(&mut self, sync_response: crate::domain::synchronization::SyncResponseView) {
        let rooms_service = self.build_rooms_service();
        for (room_id, join_entry) in &sync_response.rooms.join {
            let _ = self
                .runtime
                .block_on(rooms_service.upsert_cached_room(room_id, None, None));
            if !join_entry.state.events.is_empty() {
                self.update_room_policy_profile_from_state(room_id, &join_entry.state.events);
            }
        }
        if !sync_response.rooms.join.is_empty() {
            self.load_rooms();
        }

        let selected_room_id = self.selected_room().map(|room| room.room_id.clone());
        if let Some(selected_room_id) = selected_room_id
            && let Some(join_entry) = sync_response.rooms.join.get(&selected_room_id)
        {
            if !join_entry.state.events.is_empty() {
                self.apply_state_delta_to_selected_room(&selected_room_id, &join_entry.state.events);
                let (name, topic) = extract_room_name_and_topic(&self.room_state_events);
                if name.is_some() || topic.is_some() {
                    let _ = self.runtime.block_on(
                        rooms_service.upsert_cached_room(&selected_room_id, name, topic),
                    );
                    self.load_rooms();
                    let _ = self.set_selected_room_index_by_id(&selected_room_id);
                }
            }
            if !join_entry.timeline.events.is_empty() {
                self.room_messages.extend(join_entry.timeline.events.clone());
                self.sort_room_messages_for_chat();
                self.room_messages_status = LoadState::Loaded;
            }
        }
    }

    fn apply_state_delta_to_selected_room(
        &mut self,
        room_id: &str,
        state_events: &[RoomStateEventView],
    ) {
        for state_event in state_events {
            if let Some(existing_index) = self.room_state_events.iter().position(|existing| {
                existing.event_type == state_event.event_type && existing.state_key == state_event.state_key
            }) {
                self.room_state_events[existing_index] = state_event.clone();
            } else {
                self.room_state_events.push(state_event.clone());
            }
        }
        self.room_state_events.sort_by(|left, right| {
            left.event_type
                .cmp(&right.event_type)
                .then_with(|| left.state_key.cmp(&right.state_key))
        });
        let room_state_events_snapshot = self.room_state_events.clone();
        self.update_room_policy_profile_from_state(room_id, &room_state_events_snapshot);
        self.room_state_status = LoadState::Loaded;
    }

    fn build_transaction_id(&mut self) -> String {
        self.next_transaction_counter = self.next_transaction_counter.saturating_add(1);
        let timestamp_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        format!(
            "client-{}-{}",
            timestamp_millis, self.next_transaction_counter
        )
    }

    fn render_welcome(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(36.0);
            ui.label(
                egui::RichText::new("GRIDSTACK")
                    .size(12.0)
                    .color(ACCENT)
                    .strong(),
            );
            ui.heading(egui::RichText::new("Secure rooms, local first").size(34.0));
            ui.label(
                egui::RichText::new("Choose a homeserver and continue with a saved account.")
                    .color(TEXT_MUTED),
            );
            ui.add_space(22.0);
        });

        egui::Frame::default()
            .fill(PANEL)
            .stroke(egui::Stroke::new(1.0, PANEL_HOVER))
            .corner_radius(egui::CornerRadius::same(18))
            .inner_margin(egui::Margin::symmetric(22, 20))
            .show(ui, |ui| {
                ui.set_max_width(620.0);
                ui.label(
                    egui::RichText::new("Server")
                        .size(13.0)
                        .color(TEXT_MUTED)
                        .strong(),
                );
                ui.horizontal(|ui| {
                    if !self.servers.is_empty() {
                        let mut changed_to_index: Option<usize> = None;
                        egui::ComboBox::from_id_salt("welcome_server")
                            .width(440.0)
                            .selected_text(
                                self.servers
                                    .get(self.selected_server_index)
                                    .cloned()
                                    .unwrap_or_else(|| "Choose server".to_owned()),
                            )
                            .show_ui(ui, |ui| {
                                for (index, server) in self.servers.iter().enumerate() {
                                    if ui
                                        .selectable_label(self.selected_server_index == index, server)
                                        .clicked()
                                    {
                                        changed_to_index = Some(index);
                                    }
                                }
                            });
                        if let Some(index) = changed_to_index {
                            self.selected_server_index = index;
                            if let Some(server) = self.servers.get(index) {
                                self.server_url_input = server.clone();
                            }
                            self.persist_selected_server();
                            self.reload_accounts_for_selected_server();
                        }
                    } else {
                        ui.label(egui::RichText::new("No saved servers").color(TEXT_MUTED));
                    }

                    if ui.button("+ Server").clicked() {
                        self.show_add_server_form = true;
                    }
                });

                if self.show_add_server_form {
                    ui.add_space(10.0);
                    egui::Frame::default()
                        .fill(PANEL_SOFT)
                        .corner_radius(egui::CornerRadius::same(14))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.label("Add server");
                            ui.add(
                                egui::TextEdit::singleline(&mut self.new_server_url_input)
                                    .hint_text("100.x.y.z:3000 or http://100.x.y.z:3000"),
                            );
                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() {
                                    self.add_server();
                                }
                                if ui.button("Cancel").clicked() {
                                    self.show_add_server_form = false;
                                }
                            });
                        });
                }

                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("Account")
                        .size(13.0)
                        .color(TEXT_MUTED)
                        .strong(),
                );
                if !self.accounts_for_server.is_empty() {
                    let mut changed_account_index: Option<usize> = None;
                    egui::ComboBox::from_id_salt("welcome_account")
                        .width(440.0)
                        .selected_text(
                            self.accounts_for_server
                                .get(self.selected_account_index)
                                .map(|account| account.username.clone())
                                .unwrap_or_else(|| "Choose account".to_owned()),
                        )
                        .show_ui(ui, |ui| {
                            for (index, account) in self.accounts_for_server.iter().enumerate() {
                                if ui
                                    .selectable_label(
                                        self.selected_account_index == index,
                                        &account.username,
                                    )
                                    .clicked()
                                {
                                    changed_account_index = Some(index);
                                }
                            }
                        });
                    if let Some(index) = changed_account_index {
                        self.selected_account_index = index;
                        if let Some(account) = self.accounts_for_server.get(index) {
                            self.username_input = account.username.clone();
                            self.password_input = account.password.clone();
                        }
                        self.persist_selected_account();
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Login").clicked() {
                            self.run_login_selected_account();
                        }
                        if ui.button("Register new account").clicked() {
                            self.start_registration();
                        }
                    });
                } else {
                    ui.label(egui::RichText::new("No saved accounts for this server.").color(TEXT_MUTED));
                    ui.add_space(8.0);
                    ui.add(egui::TextEdit::singleline(&mut self.username_input).hint_text("Username"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.password_input)
                            .password(true)
                            .hint_text("Password"),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Save account").clicked() {
                            self.save_selected_account_credentials();
                        }
                        if ui.button("Register").clicked() {
                            self.start_registration();
                        }
                    });
                }
            });
    }

    fn render_choose_flow(&mut self, ui: &mut egui::Ui) {
        ui.heading("Choose Auth Flow");
        ui.label(format!("Server: {}", self.selected_server().unwrap_or("-")));

        egui::ComboBox::from_label("Available flows")
            .selected_text(
                self.available_auth_flows
                    .get(self.selected_auth_flow_index)
                    .cloned()
                    .unwrap_or_else(|| "m.login.password".to_owned()),
            )
            .show_ui(ui, |ui| {
                for (index, flow) in self.available_auth_flows.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_auth_flow_index, index, flow);
                }
            });

        ui.horizontal(|ui| {
            if ui.button("Continue").clicked() {
                self.screen = Screen::RegisterForm;
            }
            if ui.button("Back").clicked() {
                self.screen = Screen::Welcome;
            }
        });
    }

    fn render_register_form(&mut self, ui: &mut egui::Ui) {
        ui.heading("Register");
        ui.horizontal(|ui| {
            ui.label("Username");
            let response = ui.text_edit_singleline(&mut self.username_input);
            if response.changed() {
                self.username_state = UsernameState::Unknown;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Password");
            ui.add(egui::TextEdit::singleline(&mut self.password_input).password(true));
        });

        ui.horizontal(|ui| {
            if ui.button("Check username").clicked() {
                self.check_username();
            }

            match &self.username_state {
                UsernameState::Unknown => {
                    ui.label("Not checked");
                }
                UsernameState::Available => {
                    ui.colored_label(egui::Color32::GREEN, "Available");
                }
                UsernameState::Unavailable(reason) => {
                    ui.colored_label(egui::Color32::RED, reason);
                }
            }
        });

        let can_register = self.username_state == UsernameState::Available;
        if ui
            .add_enabled(can_register, egui::Button::new("Register"))
            .clicked()
        {
            self.submit_registration();
        }

        if ui.button("Back").clicked() {
            self.screen = Screen::ChooseFlow;
        }
    }

    fn render_workspace(&mut self, context: &egui::Context) {
        egui::SidePanel::left("room_sidebar")
            .resizable(true)
            .default_width(330.0)
            .frame(
                egui::Frame::default()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(14, 16)),
            )
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Gridstack")
                            .size(24.0)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Logout").clicked() {
                            self.run_logout();
                        }
                    });
                });
                if let Some(whoami) = self.whoami.as_ref() {
                    ui.label(egui::RichText::new(&whoami.user_id).color(TEXT_MUTED));
                }
                ui.add_space(16.0);

                egui::Frame::default()
                    .fill(PANEL_SOFT)
                    .corner_radius(egui::CornerRadius::same(16))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Session").color(TEXT_MUTED).strong());
                        let mut changed_server_index: Option<usize> = None;
                        egui::ComboBox::from_id_salt("workspace_server")
                            .width(ui.available_width())
                            .selected_text(
                                self.servers
                                    .get(self.selected_server_index)
                                    .cloned()
                                    .unwrap_or_else(|| "Choose server".to_owned()),
                            )
                            .show_ui(ui, |ui| {
                                for (index, server) in self.servers.iter().enumerate() {
                                    if ui
                                        .selectable_label(self.selected_server_index == index, server)
                                        .clicked()
                                    {
                                        changed_server_index = Some(index);
                                    }
                                }
                            });
                        if let Some(index) = changed_server_index {
                            self.selected_server_index = index;
                            self.persist_selected_server();
                            self.reload_accounts_for_selected_server();
                            self.load_rooms();
                        }

                        let mut changed_account_index: Option<usize> = None;
                        egui::ComboBox::from_id_salt("workspace_account")
                            .width(ui.available_width())
                            .selected_text(
                                self.accounts_for_server
                                    .get(self.selected_account_index)
                                    .map(|account| account.username.clone())
                                    .unwrap_or_else(|| "Choose account".to_owned()),
                            )
                            .show_ui(ui, |ui| {
                                for (index, account) in self.accounts_for_server.iter().enumerate() {
                                    if ui
                                        .selectable_label(
                                            self.selected_account_index == index,
                                            &account.username,
                                        )
                                        .clicked()
                                    {
                                        changed_account_index = Some(index);
                                    }
                                }
                            });
                        if let Some(index) = changed_account_index {
                            self.selected_account_index = index;
                            if let Some(account) = self.accounts_for_server.get(index) {
                                self.username_input = account.username.clone();
                                self.password_input = account.password.clone();
                            }
                            self.persist_selected_account();
                        }

                        if ui.button("Refresh cached rooms").clicked() {
                            self.load_rooms();
                        }
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Sync interval (s)").color(TEXT_MUTED));
                            ui.add(
                                egui::DragValue::new(&mut self.sync_interval_seconds)
                                    .range(0.5..=30.0)
                                    .speed(0.2),
                            );
                        });
                        ui.collapsing("Sync filter", |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.sync_filter_definition_input)
                                    .desired_rows(3)
                                    .hint_text("{\"room\":{\"timeline\":{\"limit\":30}}}"),
                            );
                            ui.horizontal(|ui| {
                                if ui.button("Define filter").clicked() {
                                    self.define_sync_filter();
                                }
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.sync_filter_id_input)
                                        .hint_text("filter id"),
                                );
                                if ui.button("Get filter").clicked() {
                                    self.fetch_sync_filter();
                                }
                            });
                            if let Some(sync_filter_lookup_output) = self.sync_filter_lookup_output.as_ref()
                            {
                                ui.monospace(pretty_json(sync_filter_lookup_output));
                            }
                        });
                    });

                ui.add_space(12.0);
                ui.collapsing("Join by room id", |ui| {
                    ui.text_edit_singleline(&mut self.join_room_id_input);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.join_reason_input)
                            .hint_text("Reason, optional"),
                    );
                    if ui.button("Join").clicked() {
                        self.join_room_by_id();
                    }
                });

                if ui
                    .button(if self.show_create_room_form {
                        "Hide create room"
                    } else {
                        "Create room"
                    })
                    .clicked()
                {
                    self.show_create_room_form = !self.show_create_room_form;
                }
                if self.show_create_room_form {
                    egui::Frame::default()
                        .fill(PANEL_SOFT)
                        .corner_radius(egui::CornerRadius::same(14))
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                        ui.add(egui::TextEdit::singleline(&mut self.new_room_name).hint_text("Room name"));
                        ui.add(egui::TextEdit::singleline(&mut self.new_room_topic).hint_text("Topic"));

                        let visibility_options = Self::visibility_options();
                        egui::ComboBox::from_label("Visibility")
                            .selected_text(
                                visibility_options
                                    .get(self.new_room_visibility_index)
                                    .copied()
                                    .unwrap_or("private"),
                            )
                            .show_ui(ui, |ui| {
                                for (index, value) in visibility_options.iter().enumerate() {
                                    if ui
                                        .selectable_value(&mut self.new_room_visibility_index, index, *value)
                                        .clicked()
                                    {
                                        self.enforce_preset_for_visibility();
                                    }
                                }
                            });

                        let active_visibility = visibility_options
                            .get(self.new_room_visibility_index)
                            .copied()
                            .unwrap_or("private");
                        let preset_options =
                            Self::available_presets_for_visibility(active_visibility);
                        if self.new_room_preset_index >= preset_options.len() {
                            self.new_room_preset_index = 0;
                        }
                        egui::ComboBox::from_label("Preset")
                            .selected_text(
                                preset_options
                                    .get(self.new_room_preset_index)
                                    .copied()
                                    .unwrap_or("private_chat"),
                            )
                            .show_ui(ui, |ui| {
                                for (index, value) in preset_options.iter().enumerate() {
                                    if ui
                                        .selectable_value(&mut self.new_room_preset_index, index, *value)
                                        .clicked()
                                    {
                                        self.enforce_visibility_for_preset();
                                    }
                                }
                            });
                        ui.checkbox(&mut self.new_room_is_direct, "Direct room");
                        if ui.button("Create").clicked() {
                            self.create_room();
                        }
                    });
                }

                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Rooms")
                            .size(13.0)
                            .color(TEXT_MUTED)
                            .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(self.rooms.len().to_string()).color(ACCENT));
                    });
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for index in 0..self.rooms.len() {
                        let room = &self.rooms[index];
                        let room_id_for_click = room.room_id.clone();
                        let room_policy_profile = self.room_policy_profiles.get(&room.room_id);
                        let room_title = room.name.clone().unwrap_or_else(|| room.room_id.clone());
                        let topic = room.topic.clone().unwrap_or_else(|| room.room_id.clone());
                        let selected = self.selected_room_index == Some(index);
                        let fill = if selected { PANEL_HOVER } else { PANEL };
                        let response = egui::Frame::default()
                            .fill(fill)
                            .stroke(egui::Stroke::new(
                                1.0,
                                if selected { ACCENT } else { PANEL_SOFT },
                            ))
                            .corner_radius(egui::CornerRadius::same(14))
                            .inner_margin(egui::Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    avatar(ui, room_title.chars().next().unwrap_or('#'), selected);
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(room_title).strong());
                                        if let Some(room_policy_profile) = room_policy_profile {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    egui::RichText::new(room_policy_profile.visibility.clone())
                                                        .size(10.0)
                                                        .color(ACCENT_STRONG),
                                                );
                                                ui.label(
                                                    egui::RichText::new(room_policy_profile.preset.clone())
                                                        .size(10.0)
                                                        .color(TEXT_MUTED),
                                                );
                                                ui.label(
                                                    egui::RichText::new(if room_policy_profile.is_direct {
                                                        "direct"
                                                    } else {
                                                        "group"
                                                    })
                                                    .size(10.0)
                                                    .color(TEXT_MUTED),
                                                );
                                            });
                                        } else {
                                            ui.label(
                                                egui::RichText::new("policy unknown")
                                                    .size(10.0)
                                                    .color(TEXT_MUTED),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new(compact_text(&topic, 38))
                                                .size(11.0)
                                                .color(TEXT_MUTED),
                                        );
                                    });
                                });
                            })
                            .response;
                        if response.interact(egui::Sense::click()).clicked() {
                            self.select_room_by_id(&room_id_for_click);
                        }
                        ui.add_space(6.0);
                    }
                });

                if ui.button("Leave selected room").clicked() {
                    self.leave_selected_room();
                }
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(BACKGROUND))
            .show(context, |ui| {
            let selected_room_id = self.selected_room().map(|room| room.room_id.clone());
            let selected_room_name = self
                .selected_room()
                .and_then(|room| room.name.clone())
                .unwrap_or_else(|| selected_room_id.clone().unwrap_or_else(|| "-".to_owned()));
            let selected_room_policy_profile = self.selected_room_policy_profile().cloned();

            egui::Frame::default()
                .fill(PANEL)
                .inner_margin(egui::Margin::symmetric(20, 14))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        avatar(ui, selected_room_name.chars().next().unwrap_or('#'), true);
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(selected_room_name)
                                    .size(20.0)
                                    .strong(),
                            );
                            if let Some(selected_room_policy_profile) = selected_room_policy_profile.as_ref() {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        egui::RichText::new(selected_room_policy_profile.visibility.clone())
                                            .size(11.0)
                                            .color(ACCENT_STRONG),
                                    );
                                    ui.label(
                                        egui::RichText::new(selected_room_policy_profile.preset.clone())
                                            .size(11.0)
                                            .color(TEXT_MUTED),
                                    );
                                    ui.label(
                                        egui::RichText::new(if selected_room_policy_profile.is_direct {
                                            "direct"
                                        } else {
                                            "group"
                                        })
                                        .size(11.0)
                                        .color(TEXT_MUTED),
                                    );
                                });
                            }
                            if let Some(room_id) = selected_room_id.as_ref() {
                                ui.label(
                                    egui::RichText::new(compact_text(room_id, 84))
                                        .size(11.0)
                                        .color(TEXT_MUTED),
                                );
                            } else {
                                ui.label(egui::RichText::new("Select a room").color(TEXT_MUTED));
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut self.sync_enabled, "Auto sync");
                            if ui.button("Sync now").clicked() {
                                self.run_single_sync_poll();
                            }
                            if ui.button("Mark read").clicked() {
                                self.mark_selected_room_as_read();
                            }
                            if ui.button("Members").clicked() {
                                self.load_selected_room_members();
                            }
                            if ui.button("Member events").clicked() {
                                self.load_selected_room_members_with_filter();
                            }
                            if ui
                                .add_enabled(
                                    self.next_messages_from_token.is_some(),
                                    egui::Button::new("Older"),
                                )
                                .clicked()
                            {
                                self.load_older_messages();
                            }
                            if ui.button("Messages").clicked() {
                                self.load_selected_room_messages(None, true);
                            }
                            if ui.button("State").clicked() {
                                self.load_selected_room_state();
                            }
                        });
                    });
                });

            ui.add_space(12.0);
            egui::Frame::default()
                .fill(PANEL_SOFT)
                .corner_radius(egui::CornerRadius::same(16))
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    ui.collapsing("Room tools", |ui| {
                        ui.label(egui::RichText::new("Invite user").color(TEXT_MUTED));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.invite_user_id_input)
                                .hint_text("@user:server"),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.invite_reason_input)
                                .hint_text("Reason, optional"),
                        );
                        if ui.button("Invite").clicked() {
                            self.invite_user_to_selected_room();
                        }
                        if let Some(policy_profile) = self.selected_room_policy_profile() {
                            if policy_profile.visibility == "private" && !policy_profile.can_invite {
                                ui.label(
                                    egui::RichText::new("Private room policy: only owner may invite.")
                                        .size(10.0)
                                        .color(TEXT_MUTED),
                                );
                            }
                        }
                        ui.separator();

                        ui.label(egui::RichText::new("Send receipt").color(TEXT_MUTED));
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("receipt_type")
                                .selected_text(
                                    Self::receipt_type_options()
                                        .get(self.receipt_type_index)
                                        .copied()
                                        .unwrap_or("m.read"),
                                )
                                .show_ui(ui, |ui| {
                                    for (index, receipt_type) in
                                        Self::receipt_type_options().iter().enumerate()
                                    {
                                        ui.selectable_value(
                                            &mut self.receipt_type_index,
                                            index,
                                            *receipt_type,
                                        );
                                    }
                                });
                            ui.add(
                                egui::TextEdit::singleline(&mut self.receipt_event_id_input)
                                    .hint_text("$event_id (optional: latest loaded)"),
                            );
                        });
                        ui.add(
                            egui::TextEdit::singleline(&mut self.receipt_thread_id_input)
                                .hint_text("thread_id optional"),
                        );
                        if ui.button("Send receipt").clicked() {
                            self.send_receipt_for_selected_room();
                        }
                        ui.separator();

                        ui.label(egui::RichText::new("Fetch event by id").color(TEXT_MUTED));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.event_lookup_input)
                                .hint_text("$event_id"),
                        );
                        if ui.button("Fetch event").clicked() {
                            self.lookup_room_event();
                        }
                        if let Some(event) = self.looked_up_event.as_ref() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} by {}",
                                    event.event_type, event.sender
                                ))
                                .color(ACCENT_STRONG),
                            );
                            ui.monospace(pretty_json(&event.content));
                        }
                        ui.separator();

                        ui.label(egui::RichText::new("State by type/key").color(TEXT_MUTED));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.state_event_type_input)
                                .hint_text("m.room.name"),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.state_key_input)
                                .hint_text("state key (empty allowed for read)"),
                        );
                        ui.horizontal(|ui| {
                            if ui.button("Get content").clicked() {
                                self.lookup_state_with_key(false);
                            }
                            if ui.button("Get full event").clicked() {
                                self.lookup_state_with_key(true);
                            }
                        });
                        ui.add(
                            egui::TextEdit::multiline(&mut self.state_content_input)
                                .hint_text("{\"name\":\"Room\"}")
                                .desired_rows(4),
                        );
                        if ui.button("Set state with key").clicked() {
                            self.set_state_with_key();
                        }
                        if let Some(policy_profile) = self.selected_room_policy_profile() {
                            if policy_profile.preset == "private_chat" && !policy_profile.can_rename_name
                            {
                                ui.label(
                                    egui::RichText::new(
                                        "private_chat policy: only owner may rename room name.",
                                    )
                                    .size(10.0)
                                    .color(TEXT_MUTED),
                                );
                            }
                        }
                        if let Some(state_value) = self.looked_up_state_value.as_ref() {
                            ui.label(egui::RichText::new("Fetched state value").color(ACCENT_STRONG));
                            ui.monospace(pretty_json(state_value));
                        }
                    });

                    ui.separator();
            ui.collapsing("Room state", |ui| match &self.room_state_status {
                LoadState::Idle => {
                    ui.label(egui::RichText::new("No room selected.").color(TEXT_MUTED));
                }
                LoadState::Loading => {
                    ui.label(egui::RichText::new("Loading room state...").color(TEXT_MUTED));
                }
                LoadState::Error(error) => {
                    ui.colored_label(DANGER, format!("Failed to load state: {error}"));
                }
                LoadState::Loaded => {
                    if self.room_state_events.is_empty() {
                        ui.label(egui::RichText::new("This room has no state events.").color(TEXT_MUTED));
                    } else {
                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            for event in &self.room_state_events {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} [{}]",
                                        event.event_type, event.state_key
                                    ))
                                    .color(ACCENT_STRONG)
                                    .strong(),
                                );
                                ui.label(egui::RichText::new(&event.sender).color(TEXT_MUTED));
                                ui.monospace(pretty_json(&event.content));
                                ui.separator();
                            }
                        });
                    }
                }
            });
                });

            ui.add_space(8.0);
            egui::Frame::default()
                .fill(PANEL_SOFT)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                ui.label(egui::RichText::new("Joined members").color(TEXT_MUTED).strong());
                    match &self.room_members_status {
                        LoadState::Idle => {
                            ui.label(egui::RichText::new("Use Members button to load.").color(TEXT_MUTED));
                        }
                        LoadState::Loading => {
                            ui.label(egui::RichText::new("Loading members...").color(TEXT_MUTED));
                        }
                        LoadState::Error(error) => {
                            ui.colored_label(DANGER, format!("Failed to load members: {error}"));
                        }
                        LoadState::Loaded => {
                            if self.room_members.is_empty() {
                                ui.label(egui::RichText::new("No joined members returned.").color(TEXT_MUTED));
                            } else {
                                egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                                    for member in &self.room_members {
                                        let display_name = member
                                            .display_name
                                            .clone()
                                            .unwrap_or_else(|| member.user_id.clone());
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(display_name).color(ACCENT_STRONG));
                                            ui.label(egui::RichText::new(&member.user_id).color(TEXT_MUTED));
                                            if let Some(avatar_url) = member.avatar_url.as_ref() {
                                                ui.label(egui::RichText::new(avatar_url).color(TEXT_MUTED));
                                            }
                                        });
                                    }
                                });
                            }
                        }
                    }
                });

            ui.add_space(8.0);
            egui::Frame::default()
                .fill(PANEL_SOFT)
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Member events").color(TEXT_MUTED).strong());
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.room_membership_filter_input)
                                .hint_text("membership filter: join/invite/leave/ban/knock"),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut self.room_not_membership_filter_input)
                                .hint_text("not_membership"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.room_members_at_token_input)
                                .hint_text("at token"),
                        );
                        if ui.button("Load member events").clicked() {
                            self.load_selected_room_members_with_filter();
                        }
                    });
                    match &self.room_members_timeline_status {
                        LoadState::Idle => {
                            ui.label(
                                egui::RichText::new("Load `/rooms/{roomId}/members` with filters.")
                                    .color(TEXT_MUTED),
                            );
                        }
                        LoadState::Loading => {
                            ui.label(egui::RichText::new("Loading member events...").color(TEXT_MUTED));
                        }
                        LoadState::Error(error) => {
                            ui.colored_label(DANGER, format!("Failed to load member events: {error}"));
                        }
                        LoadState::Loaded => {
                            if self.room_members_timeline_events.is_empty() {
                                ui.label(
                                    egui::RichText::new("No member events returned.").color(TEXT_MUTED),
                                );
                            } else {
                                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                                    for event in &self.room_members_timeline_events {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} [{}]",
                                                event.event_type, event.state_key
                                            ))
                                            .color(ACCENT_STRONG)
                                            .strong(),
                                        );
                                        ui.label(egui::RichText::new(&event.sender).color(TEXT_MUTED));
                                        ui.monospace(pretty_json(&event.content));
                                        ui.separator();
                                    }
                                });
                            }
                        }
                    }
                });

            ui.add_space(12.0);
            ui.label(
                egui::RichText::new("Messages")
                    .size(14.0)
                    .color(TEXT_MUTED)
                    .strong(),
            );
            match &self.room_messages_status {
                LoadState::Idle => {
                    empty_chat(ui, "Select a room from the sidebar to inspect its timeline.");
                }
                LoadState::Loading => {
                    empty_chat(ui, "Loading room messages...");
                }
                LoadState::Error(error) => {
                    ui.colored_label(DANGER, format!("Failed to load messages: {error}"));
                }
                LoadState::Loaded => {
                    if self.room_messages.is_empty() {
                        empty_chat(ui, "No timeline messages returned for this room.");
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for event in &self.room_messages {
                                render_timeline_event(ui, event, self.whoami.as_ref());
                                ui.add_space(10.0);
                            }
                        });
                    }
                }
            }

            ui.add_space(10.0);
            egui::Frame::default()
                .fill(PANEL)
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    let can_send = self.selected_room().is_some() && !self.composer_input.trim().is_empty();
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.composer_input)
                                .hint_text("Write a message"),
                        );
                        if ui
                            .add_enabled(can_send, egui::Button::new("Send"))
                            .clicked()
                        {
                            self.send_message();
                        }
                    });
                    if self.selected_room().is_none() {
                        ui.label(
                            egui::RichText::new("Select a room to enable message sending.")
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(
                                "Uses `PUT /rooms/{roomId}/send/m.room.message/{txnId}`.",
                            )
                            .size(11.0)
                            .color(TEXT_MUTED),
                        );
                    }
                });
        });
    }

    fn visibility_options() -> &'static [&'static str] {
        &["private", "public"]
    }

    fn receipt_type_options() -> &'static [&'static str] {
        &["m.read", "m.read.private", "m.fully_read"]
    }

    fn available_presets_for_visibility(visibility: &str) -> &'static [&'static str] {
        match visibility {
            "public" => &["public_chat"],
            _ => &["private_chat", "trusted_private_chat"],
        }
    }

    fn enforce_preset_for_visibility(&mut self) {
        let visibility = Self::visibility_options()
            .get(self.new_room_visibility_index)
            .copied()
            .unwrap_or("private");
        let presets = Self::available_presets_for_visibility(visibility);
        if self.new_room_preset_index >= presets.len() {
            self.new_room_preset_index = 0;
        }
    }

    fn enforce_visibility_for_preset(&mut self) {
        let private_presets = ["private_chat", "trusted_private_chat"];
        let current_visibility = Self::visibility_options()
            .get(self.new_room_visibility_index)
            .copied()
            .unwrap_or("private");
        let preset = Self::available_presets_for_visibility(current_visibility)
            .get(self.new_room_preset_index)
            .copied()
            .unwrap_or("private_chat");
        self.new_room_visibility_index = if private_presets.contains(&preset) { 0 } else { 1 };
    }
}

impl eframe::App for ClientDesktopApplication {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.log_status_message_if_changed();
        self.run_periodic_sync_if_due();
        apply_visual_theme(context);

        match self.screen {
            Screen::Workspace => self.render_workspace(context),
            _ => {
                egui::CentralPanel::default()
                    .frame(egui::Frame::default().fill(BACKGROUND))
                    .show(context, |ui| match self.screen {
                    Screen::Welcome => self.render_welcome(ui),
                    Screen::ChooseFlow => self.render_choose_flow(ui),
                    Screen::RegisterForm => self.render_register_form(ui),
                    Screen::Workspace => {}
                });
            }
        }

        egui::TopBottomPanel::bottom("status_bar").show(context, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(&self.status_message).color(TEXT_MUTED));
            });
        });
    }
}

fn apply_visual_theme(context: &egui::Context) {
    let mut style = (*context.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = PANEL;
    style.visuals.extreme_bg_color = BACKGROUND;
    style.visuals.widgets.noninteractive.bg_fill = PANEL;
    style.visuals.widgets.inactive.bg_fill = PANEL_SOFT;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    style.visuals.widgets.hovered.bg_fill = PANEL_HOVER;
    style.visuals.widgets.active.bg_fill = ACCENT;
    style.visuals.selection.bg_fill = ACCENT;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    context.set_style(style);
}

fn avatar(ui: &mut egui::Ui, initial: char, highlighted: bool) {
    let color = if highlighted { ACCENT } else { PANEL_HOVER };
    let text_color = if highlighted {
        BACKGROUND
    } else {
        egui::Color32::WHITE
    };
    egui::Frame::default()
        .fill(color)
        .corner_radius(egui::CornerRadius::same(40))
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(initial.to_uppercase().to_string())
                    .color(text_color)
                    .strong(),
            );
        });
}

fn empty_chat(ui: &mut egui::Ui, message: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(48.0);
        ui.label(egui::RichText::new("No active timeline").size(22.0).strong());
        ui.label(egui::RichText::new(message).color(TEXT_MUTED));
    });
}

fn render_timeline_event(
    ui: &mut egui::Ui,
    event: &RoomTimelineEventView,
    whoami: Option<&WhoAmIView>,
) {
    if event.event_type != "m.room.message" {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "[event] {} {} {}",
                    event.event_type,
                    compact_text(&event.sender, 42),
                    message_body(event)
                ))
                .size(11.0)
                .color(TEXT_MUTED),
            );
        });
        return;
    }

    let sent_by_me = whoami
        .map(|view| view.user_id == event.sender)
        .unwrap_or(false);
    let fill = if sent_by_me { ACCENT } else { PANEL_SOFT };
    let text_color = if sent_by_me {
        BACKGROUND
    } else {
        egui::Color32::WHITE
    };
    let body = message_body(event);

    ui.with_layout(
        if sent_by_me {
            egui::Layout::right_to_left(egui::Align::Min)
        } else {
            egui::Layout::left_to_right(egui::Align::Min)
        },
        |ui| {
            egui::Frame::default()
                .fill(fill)
                .stroke(egui::Stroke::new(
                    1.0,
                    if sent_by_me { ACCENT } else { PANEL_HOVER },
                ))
                .corner_radius(egui::CornerRadius::same(18))
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_max_width(560.0);
                    ui.label(egui::RichText::new(body).color(text_color));
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(compact_text(&event.sender, 42))
                                .size(10.0)
                                .color(if sent_by_me { BACKGROUND } else { TEXT_MUTED }),
                        );
                        ui.label(
                            egui::RichText::new(&event.event_type)
                                .size(10.0)
                                .color(if sent_by_me { BACKGROUND } else { ACCENT_STRONG }),
                        );
                    });
                });
        },
    );
}

fn extract_room_name_and_topic(events: &[RoomStateEventView]) -> (Option<String>, Option<String>) {
    let mut room_name: Option<String> = None;
    let mut room_topic: Option<String> = None;

    for event in events {
        if event.state_key.is_empty() && event.event_type == "m.room.name" {
            if let Some(name) = event.content.get("name").and_then(serde_json::Value::as_str) {
                room_name = Some(name.to_owned());
            }
        }
        if event.state_key.is_empty() && event.event_type == "m.room.topic" {
            if let Some(topic) = event.content.get("topic").and_then(serde_json::Value::as_str) {
                room_topic = Some(topic.to_owned());
            }
        }
    }

    (room_name, room_topic)
}

fn derive_room_policy_profile(
    events: &[RoomStateEventView],
    whoami_user_id: Option<&str>,
) -> RoomPolicyProfile {
    let mut owner_user_id: Option<String> = None;
    let mut visibility = "private".to_owned();
    let mut is_direct = false;
    let mut users_default_level = 0_i64;
    let mut invite_required_level = 0_i64;
    let mut room_name_required_level = 50_i64;
    let mut user_levels: BTreeMap<String, i64> = BTreeMap::new();

    for event in events {
        if event.event_type == "m.room.create" && event.state_key.is_empty() {
            owner_user_id = event
                .content
                .get("creator")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
        } else if event.event_type == "m.room.join_rules" && event.state_key.is_empty() {
            visibility = if event
                .content
                .get("join_rule")
                .and_then(serde_json::Value::as_str)
                == Some("public")
            {
                "public".to_owned()
            } else {
                "private".to_owned()
            };
        } else if event.event_type == "m.room.power_levels" && event.state_key.is_empty() {
            users_default_level = event
                .content
                .get("users_default")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            invite_required_level = event
                .content
                .get("invite")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            let state_default_level = event
                .content
                .get("state_default")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(50);

            if let Some(events_map) = event.content.get("events").and_then(serde_json::Value::as_object)
            {
                room_name_required_level = events_map
                    .get("m.room.name")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(state_default_level);
            } else {
                room_name_required_level = state_default_level;
            }

            if let Some(users_map) = event.content.get("users").and_then(serde_json::Value::as_object) {
                for (user_id, level) in users_map {
                    if let Some(level) = level.as_i64() {
                        user_levels.insert(user_id.clone(), level);
                    }
                }
            }
        } else if event.event_type == "m.room.member"
            && whoami_user_id.is_some()
            && event.state_key == whoami_user_id.unwrap_or_default()
        {
            is_direct = event
                .content
                .get("is_direct")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
        }
    }

    let mut preset = if visibility == "public" {
        "public_chat".to_owned()
    } else {
        "private_chat".to_owned()
    };

    if visibility == "private"
        && owner_user_id.is_some()
        && user_levels
            .iter()
            .any(|(user_id, level)| Some(user_id.as_str()) != owner_user_id.as_deref() && *level >= 100)
    {
        preset = "trusted_private_chat".to_owned();
    }

    let current_user_level = whoami_user_id
        .and_then(|user_id| user_levels.get(user_id).copied())
        .unwrap_or(users_default_level);
    let mut can_invite = current_user_level >= invite_required_level;
    let mut can_rename_name = current_user_level >= room_name_required_level;

    if visibility == "private"
        && let (Some(owner_user_id), Some(whoami_user_id)) =
            (owner_user_id.as_deref(), whoami_user_id)
        && owner_user_id != whoami_user_id
    {
        can_invite = false;
    }

    if preset == "private_chat" {
        can_rename_name = matches!(
            (owner_user_id.as_deref(), whoami_user_id),
            (Some(owner_user_id), Some(whoami_user_id)) if owner_user_id == whoami_user_id
        );
    } else if preset == "trusted_private_chat" {
        can_rename_name = whoami_user_id.is_some();
    }

    RoomPolicyProfile {
        visibility,
        preset,
        is_direct,
        owner_user_id,
        can_invite,
        can_rename_name,
    }
}

fn pretty_json(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn normalize_server_url_input(input: &str) -> Result<String, String> {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Server URL required".to_owned());
    }

    let with_scheme = if trimmed.contains("://") {
        trimmed.to_owned()
    } else {
        format!("http://{trimmed}")
    };

    match with_scheme.parse::<http::Uri>() {
        Ok(uri) => {
            if uri.scheme_str().is_none() || uri.authority().is_none() {
                return Err(
                    "Invalid server URL. Use format like http://100.x.y.z:3000".to_owned(),
                );
            }
            Ok(with_scheme)
        }
        Err(_) => Err("Invalid server URL. Use format like http://100.x.y.z:3000".to_owned()),
    }
}

fn message_body(event: &RoomTimelineEventView) -> String {
    event
        .content
        .get("body")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| pretty_json(&event.content))
}

fn compact_text(value: &str, maximum_characters: usize) -> String {
    if value.chars().count() <= maximum_characters {
        return value.to_owned();
    }

    let mut shortened = value
        .chars()
        .take(maximum_characters.saturating_sub(3))
        .collect::<String>();
    shortened.push_str("...");
    shortened
}

fn append_status_message_to_log_file(status_message: &str) {
    let log_file_path = resolve_client_log_file_path();
    if let Some(parent_directory) = log_file_path.parent() {
        let _ = fs::create_dir_all(parent_directory);
    }

    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
    {
        let unix_timestamp_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{unix_timestamp_seconds}] UI status: {status_message}");
    }
}

fn resolve_client_log_file_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            return Path::new(&local_app_data)
                .join("GridstackClient")
                .join("logs")
                .join("client.log");
        }

        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            return Path::new(&user_profile)
                .join("AppData")
                .join("Local")
                .join("GridstackClient")
                .join("logs")
                .join("client.log");
        }
    }

    if let Ok(current_directory) = std::env::current_dir() {
        return current_directory
            .join("generated")
            .join("logs")
            .join("client-app.log");
    }

    PathBuf::from("client.log")
}
